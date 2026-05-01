use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver, Sender};
use egui::Color32;

use super::ConsoleStatus;

pub struct SerialCell {
    pub ch: String,
    pub fg: Color32,
    pub bg: Color32,
    pub bold: bool,
}

pub struct SerialScreen {
    pub rows: u16,
    pub cols: u16,
    pub cells: Vec<Vec<SerialCell>>,
    pub cursor: (u16, u16),
}

pub struct SerialHandle {
    screen: Arc<Mutex<SerialScreen>>,
    dirty: Arc<AtomicBool>,
    input_tx: Sender<Vec<u8>>,
    status: Arc<Mutex<ConsoleStatus>>,
    stop: Arc<AtomicBool>,
}

impl SerialHandle {
    pub fn connect(uri: &str, domain_name: &str) -> Self {
        let rows: u16 = 24;
        let cols: u16 = 80;
        let screen = Arc::new(Mutex::new(make_empty_screen(rows, cols)));
        let dirty = Arc::new(AtomicBool::new(false));
        let status = Arc::new(Mutex::new(ConsoleStatus::Connecting));
        let stop = Arc::new(AtomicBool::new(false));
        let (input_tx, input_rx) = crossbeam_channel::unbounded();

        let sc = screen.clone();
        let d = dirty.clone();
        let st = status.clone();
        let sp = stop.clone();
        let uri = uri.to_string();
        let name = domain_name.to_string();

        std::thread::Builder::new()
            .name("serial-console".into())
            .spawn(move || {
                serial_loop(uri, name, rows, cols, sc, d, input_rx, st, sp);
            })
            .expect("Failed to spawn serial thread");

        Self {
            screen,
            dirty,
            input_tx,
            status,
            stop,
        }
    }

    pub fn status(&self) -> ConsoleStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn is_connected(&self) -> bool {
        self.status() == ConsoleStatus::Connected
    }

    pub fn take_screen_if_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::Relaxed)
    }

    pub fn screen(&self) -> std::sync::MutexGuard<'_, SerialScreen> {
        self.screen.lock().unwrap()
    }

    pub fn send_input(&self, bytes: &[u8]) {
        let _ = self.input_tx.send(bytes.to_vec());
    }

    pub fn send_key(&self, key: egui::Key) {
        let bytes: &[u8] = match key {
            egui::Key::Enter => b"\r",
            egui::Key::Backspace => b"\x7f",
            egui::Key::Tab => b"\t",
            egui::Key::Escape => b"\x1b",
            egui::Key::ArrowUp => b"\x1b[A",
            egui::Key::ArrowDown => b"\x1b[B",
            egui::Key::ArrowRight => b"\x1b[C",
            egui::Key::ArrowLeft => b"\x1b[D",
            egui::Key::Home => b"\x1b[H",
            egui::Key::End => b"\x1b[F",
            egui::Key::PageUp => b"\x1b[5~",
            egui::Key::PageDown => b"\x1b[6~",
            egui::Key::Delete => b"\x1b[3~",
            egui::Key::Insert => b"\x1b[2~",
            _ => return,
        };
        self.send_input(bytes);
    }

    pub fn disconnect(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for SerialHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn serial_loop(
    uri: String,
    domain_name: String,
    rows: u16,
    cols: u16,
    screen: Arc<Mutex<SerialScreen>>,
    dirty: Arc<AtomicBool>,
    input_rx: Receiver<Vec<u8>>,
    status: Arc<Mutex<ConsoleStatus>>,
    stop: Arc<AtomicBool>,
) {
    let mut child = match spawn_virsh_console(&uri, &domain_name) {
        Ok(c) => c,
        Err(e) => {
            *status.lock().unwrap() = ConsoleStatus::Error(format!("Failed to start: {e}"));
            return;
        }
    };

    *status.lock().unwrap() = ConsoleStatus::Connected;

    let mut stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            *status.lock().unwrap() = ConsoleStatus::Error("No stdout".into());
            return;
        }
    };
    let stdin = child.stdin.take();

    let input_stop = stop.clone();
    let stdin_thread = std::thread::spawn(move || {
        let mut stdin = match stdin {
            Some(s) => s,
            None => return,
        };
        while !input_stop.load(Ordering::Relaxed) {
            match input_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(data) => {
                    let _ = stdin.write_all(&data);
                    let _ = stdin.flush();
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    let mut parser = vt100::Parser::new(rows, cols, 0);
    let mut buf = [0u8; 4096];

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        match stdout.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                parser.process(&buf[..n]);
                sync_screen(&parser, &screen);
                dirty.store(true, Ordering::Relaxed);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(_) => break,
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    let _ = stdin_thread.join();
    *status.lock().unwrap() = ConsoleStatus::Disconnected;
}

fn spawn_virsh_console(uri: &str, domain_name: &str) -> std::io::Result<Child> {
    Command::new("script")
        .args([
            "-qc",
            &format!("virsh -c {uri} console {domain_name}"),
            "/dev/null",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
}

fn sync_screen(parser: &vt100::Parser, screen: &Arc<Mutex<SerialScreen>>) {
    let vt_screen = parser.screen();
    let (rows, cols) = vt_screen.size();
    let cursor = vt_screen.cursor_position();

    let mut cells = Vec::with_capacity(rows as usize);
    for row in 0..rows {
        let mut row_cells = Vec::with_capacity(cols as usize);
        for col in 0..cols {
            let cell = vt_screen.cell(row, col);
            let (ch, fg, bg, bold) = match cell {
                Some(c) => {
                    let contents = c.contents();
                    let ch = if contents.is_empty() {
                        " ".to_string()
                    } else {
                        contents.to_string()
                    };
                    let fg = vt100_color_to_egui(c.fgcolor(), false);
                    let bg = vt100_color_to_egui(c.bgcolor(), true);
                    (ch, fg, bg, c.bold())
                }
                None => (" ".to_string(), Color32::LIGHT_GRAY, Color32::BLACK, false),
            };
            row_cells.push(SerialCell { ch, fg, bg, bold });
        }
        cells.push(row_cells);
    }

    let mut s = screen.lock().unwrap();
    s.rows = rows;
    s.cols = cols;
    s.cells = cells;
    s.cursor = cursor;
}

fn make_empty_screen(rows: u16, cols: u16) -> SerialScreen {
    let cells = (0..rows)
        .map(|_| {
            (0..cols)
                .map(|_| SerialCell {
                    ch: " ".to_string(),
                    fg: Color32::LIGHT_GRAY,
                    bg: Color32::BLACK,
                    bold: false,
                })
                .collect()
        })
        .collect();
    SerialScreen {
        rows,
        cols,
        cells,
        cursor: (0, 0),
    }
}

fn vt100_color_to_egui(color: vt100::Color, is_bg: bool) -> Color32 {
    match color {
        vt100::Color::Default => {
            if is_bg {
                Color32::BLACK
            } else {
                Color32::LIGHT_GRAY
            }
        }
        vt100::Color::Idx(idx) => ansi_index_to_color32(idx),
        vt100::Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
    }
}

fn ansi_index_to_color32(idx: u8) -> Color32 {
    match idx {
        0 => Color32::from_rgb(0, 0, 0),
        1 => Color32::from_rgb(205, 0, 0),
        2 => Color32::from_rgb(0, 205, 0),
        3 => Color32::from_rgb(205, 205, 0),
        4 => Color32::from_rgb(0, 0, 238),
        5 => Color32::from_rgb(205, 0, 205),
        6 => Color32::from_rgb(0, 205, 205),
        7 => Color32::from_rgb(229, 229, 229),
        8 => Color32::from_rgb(127, 127, 127),
        9 => Color32::from_rgb(255, 0, 0),
        10 => Color32::from_rgb(0, 255, 0),
        11 => Color32::from_rgb(255, 255, 0),
        12 => Color32::from_rgb(92, 92, 255),
        13 => Color32::from_rgb(255, 0, 255),
        14 => Color32::from_rgb(0, 255, 255),
        15 => Color32::from_rgb(255, 255, 255),
        16..=231 => {
            let n = idx - 16;
            let b = (n % 6) * 51;
            let g = ((n / 6) % 6) * 51;
            let r = (n / 36) * 51;
            Color32::from_rgb(r, g, b)
        }
        232..=255 => {
            let gray = 8 + (idx - 232) * 10;
            Color32::from_rgb(gray, gray, gray)
        }
    }
}
