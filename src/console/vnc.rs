use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use egui::{Color32, ColorImage};
use tokio::net::TcpStream;
use vnc::{ClientKeyEvent, ClientMouseEvent, PixelFormat, VncConnector, VncEncoding, VncEvent, X11Event};

use super::ConsoleStatus;

pub enum VncInput {
    Key { keysym: u32, down: bool },
    Mouse { x: u16, y: u16, buttons: u8 },
}

pub struct VncHandle {
    framebuffer: Arc<Mutex<Option<ColorImage>>>,
    dirty: Arc<AtomicBool>,
    input_tx: Sender<VncInput>,
    status: Arc<Mutex<ConsoleStatus>>,
    stop: Arc<AtomicBool>,
}

impl VncHandle {
    pub fn connect(host: &str, port: u16, password: Option<String>) -> Self {
        let framebuffer: Arc<Mutex<Option<ColorImage>>> = Arc::new(Mutex::new(None));
        let dirty = Arc::new(AtomicBool::new(false));
        let status = Arc::new(Mutex::new(ConsoleStatus::Connecting));
        let stop = Arc::new(AtomicBool::new(false));
        let (input_tx, input_rx) = crossbeam_channel::unbounded();

        let fb = framebuffer.clone();
        let d = dirty.clone();
        let st = status.clone();
        let sp = stop.clone();
        let host = host.to_string();

        std::thread::Builder::new()
            .name("vnc-client".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(vnc_event_loop(host, port, password, fb, d, input_rx, st, sp));
            })
            .expect("Failed to spawn VNC thread");

        Self {
            framebuffer,
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

    pub fn take_framebuffer_if_dirty(&self) -> Option<ColorImage> {
        if !self.dirty.swap(false, Ordering::Relaxed) {
            return None;
        }
        self.framebuffer.lock().ok()?.clone()
    }

    pub fn framebuffer_size(&self) -> Option<[usize; 2]> {
        self.framebuffer.lock().ok()?.as_ref().map(|fb| fb.size)
    }

    pub fn send_key(&self, keysym: u32, down: bool) {
        let _ = self.input_tx.send(VncInput::Key { keysym, down });
    }

    pub fn send_mouse(&self, x: u16, y: u16, buttons: u8) {
        let _ = self.input_tx.send(VncInput::Mouse { x, y, buttons });
    }

    pub fn disconnect(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for VncHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

async fn vnc_event_loop(
    host: String,
    port: u16,
    password: Option<String>,
    framebuffer: Arc<Mutex<Option<ColorImage>>>,
    dirty: Arc<AtomicBool>,
    input_rx: Receiver<VncInput>,
    status: Arc<Mutex<ConsoleStatus>>,
    stop: Arc<AtomicBool>,
) {
    let addr = format!("{host}:{port}");
    let tcp = match TcpStream::connect(&addr).await {
        Ok(s) => s,
        Err(e) => {
            *status.lock().unwrap() = ConsoleStatus::Error(format!("Connect failed: {e}"));
            return;
        }
    };

    let pwd = password.unwrap_or_default();

    let vnc = VncConnector::new(tcp)
        .set_auth_method(async move { Ok(pwd) })
        .add_encoding(VncEncoding::Tight)
        .add_encoding(VncEncoding::Zrle)
        .add_encoding(VncEncoding::CopyRect)
        .add_encoding(VncEncoding::Raw)
        .add_encoding(VncEncoding::CursorPseudo)
        .add_encoding(VncEncoding::DesktopSizePseudo)
        .set_pixel_format(PixelFormat::rgba())
        .build();

    let vnc = match vnc {
        Ok(v) => v,
        Err(e) => {
            *status.lock().unwrap() = ConsoleStatus::Error(format!("VNC build failed: {e}"));
            return;
        }
    };

    let vnc = match vnc.try_start().await {
        Ok(v) => v,
        Err(e) => {
            *status.lock().unwrap() = ConsoleStatus::Error(format!("VNC handshake failed: {e}"));
            return;
        }
    };

    let vnc = match vnc.finish() {
        Ok(v) => v,
        Err(e) => {
            *status.lock().unwrap() = ConsoleStatus::Error(format!("VNC finish failed: {e}"));
            return;
        }
    };

    *status.lock().unwrap() = ConsoleStatus::Connected;

    if let Err(e) = vnc.input(X11Event::Refresh).await {
        *status.lock().unwrap() = ConsoleStatus::Error(format!("Refresh failed: {e}"));
        return;
    }

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        while let Ok(input) = input_rx.try_recv() {
            let result = match input {
                VncInput::Key { keysym, down } => vnc.input(X11Event::KeyEvent(
                    ClientKeyEvent {
                        keycode: keysym,
                        down,
                    },
                )).await,
                VncInput::Mouse { x, y, buttons } => vnc.input(X11Event::PointerEvent(
                    ClientMouseEvent {
                        position_x: x,
                        position_y: y,
                        bottons: buttons,
                    },
                )).await,
            };
            if let Err(e) = result {
                log::warn!("VNC input error: {e}");
            }
        }

        let event = match tokio::time::timeout(Duration::from_millis(16), vnc.recv_event()).await {
            Ok(Ok(event)) => Some(event),
            Ok(Err(e)) => {
                *status.lock().unwrap() = ConsoleStatus::Error(format!("VNC error: {e}"));
                break;
            }
            Err(_) => None,
        };

        if let Some(event) = event {
            match event {
                VncEvent::SetResolution(screen) => {
                    let w = screen.width as usize;
                    let h = screen.height as usize;
                    let mut fb = framebuffer.lock().unwrap();
                    *fb = Some(ColorImage::new([w, h], vec![Color32::BLACK; w * h]));
                    dirty.store(true, Ordering::Relaxed);
                }
                VncEvent::RawImage(rect, data) => {
                    let mut fb = framebuffer.lock().unwrap();
                    if let Some(fb) = fb.as_mut() {
                        blit_rgba(fb, &rect, &data);
                        dirty.store(true, Ordering::Relaxed);
                    }
                }
                VncEvent::Copy(src, dst) => {
                    let mut fb = framebuffer.lock().unwrap();
                    if let Some(fb) = fb.as_mut() {
                        copy_rect(fb, &src, &dst);
                        dirty.store(true, Ordering::Relaxed);
                    }
                }
                VncEvent::JpegImage(rect, data) => {
                    if let Ok(img) = image::load_from_memory_with_format(&data, image::ImageFormat::Jpeg) {
                        let rgba = img.to_rgba8();
                        let mut fb = framebuffer.lock().unwrap();
                        if let Some(fb) = fb.as_mut() {
                            blit_rgba_image(fb, &rect, &rgba);
                            dirty.store(true, Ordering::Relaxed);
                        }
                    }
                }
                VncEvent::SetCursor(_, _) | VncEvent::Bell | VncEvent::Text(_) => {}
                _ => {}
            }

            let _ = vnc.input(X11Event::Refresh).await;
        } else {
            let _ = vnc.input(X11Event::Refresh).await;
        }
    }

    let _ = vnc.close();
    *status.lock().unwrap() = ConsoleStatus::Disconnected;
}

fn blit_rgba(fb: &mut ColorImage, rect: &vnc::Rect, data: &[u8]) {
    let fb_w = fb.size[0];
    let rx = rect.x as usize;
    let ry = rect.y as usize;
    let rw = rect.width as usize;
    let rh = rect.height as usize;

    for row in 0..rh {
        for col in 0..rw {
            let src = (row * rw + col) * 4;
            if src + 2 >= data.len() {
                return;
            }
            let dst = (ry + row) * fb_w + (rx + col);
            if dst < fb.pixels.len() {
                fb.pixels[dst] = Color32::from_rgb(data[src], data[src + 1], data[src + 2]);
            }
        }
    }
}

fn blit_rgba_image(fb: &mut ColorImage, rect: &vnc::Rect, img: &image::RgbaImage) {
    let fb_w = fb.size[0];
    let rx = rect.x as usize;
    let ry = rect.y as usize;

    for (x, y, pixel) in img.enumerate_pixels() {
        let dst_x = rx + x as usize;
        let dst_y = ry + y as usize;
        let idx = dst_y * fb_w + dst_x;
        if idx < fb.pixels.len() {
            fb.pixels[idx] = Color32::from_rgb(pixel[0], pixel[1], pixel[2]);
        }
    }
}

fn copy_rect(fb: &mut ColorImage, src: &vnc::Rect, dst: &vnc::Rect) {
    let fb_w = fb.size[0];
    let w = dst.width as usize;
    let h = dst.height as usize;

    let mut buf = vec![Color32::BLACK; w * h];
    for row in 0..h {
        for col in 0..w {
            let si = (src.y as usize + row) * fb_w + (src.x as usize + col);
            if si < fb.pixels.len() {
                buf[row * w + col] = fb.pixels[si];
            }
        }
    }
    for row in 0..h {
        for col in 0..w {
            let di = (dst.y as usize + row) * fb_w + (dst.x as usize + col);
            if di < fb.pixels.len() {
                fb.pixels[di] = buf[row * w + col];
            }
        }
    }
}

pub fn key_to_keysym(key: egui::Key, shift: bool) -> Option<u32> {
    use egui::Key::*;
    Some(match key {
        ArrowDown => 0xff54,
        ArrowLeft => 0xff51,
        ArrowRight => 0xff53,
        ArrowUp => 0xff52,
        Escape => 0xff1b,
        Tab => 0xff09,
        Backspace => 0xff08,
        Enter => 0xff0d,
        Space => 0x0020,
        Insert => 0xff63,
        Delete => 0xffff,
        Home => 0xff50,
        End => 0xff57,
        PageUp => 0xff55,
        PageDown => 0xff56,

        A => if shift { 0x41 } else { 0x61 },
        B => if shift { 0x42 } else { 0x62 },
        C => if shift { 0x43 } else { 0x63 },
        D => if shift { 0x44 } else { 0x64 },
        E => if shift { 0x45 } else { 0x65 },
        F => if shift { 0x46 } else { 0x66 },
        G => if shift { 0x47 } else { 0x67 },
        H => if shift { 0x48 } else { 0x68 },
        I => if shift { 0x49 } else { 0x69 },
        J => if shift { 0x4a } else { 0x6a },
        K => if shift { 0x4b } else { 0x6b },
        L => if shift { 0x4c } else { 0x6c },
        M => if shift { 0x4d } else { 0x6d },
        N => if shift { 0x4e } else { 0x6e },
        O => if shift { 0x4f } else { 0x6f },
        P => if shift { 0x50 } else { 0x70 },
        Q => if shift { 0x51 } else { 0x71 },
        R => if shift { 0x52 } else { 0x72 },
        S => if shift { 0x53 } else { 0x73 },
        T => if shift { 0x54 } else { 0x74 },
        U => if shift { 0x55 } else { 0x75 },
        V => if shift { 0x56 } else { 0x76 },
        W => if shift { 0x57 } else { 0x77 },
        X => if shift { 0x58 } else { 0x78 },
        Y => if shift { 0x59 } else { 0x79 },
        Z => if shift { 0x5a } else { 0x7a },

        Num0 => if shift { 0x29 } else { 0x30 }, // )
        Num1 => if shift { 0x21 } else { 0x31 }, // !
        Num2 => if shift { 0x40 } else { 0x32 }, // @
        Num3 => if shift { 0x23 } else { 0x33 }, // #
        Num4 => if shift { 0x24 } else { 0x34 }, // $
        Num5 => if shift { 0x25 } else { 0x35 }, // %
        Num6 => if shift { 0x5e } else { 0x36 }, // ^
        Num7 => if shift { 0x26 } else { 0x37 }, // &
        Num8 => if shift { 0x2a } else { 0x38 }, // *
        Num9 => if shift { 0x28 } else { 0x39 }, // (

        Minus => if shift { 0x5f } else { 0x2d },         // _ / -
        Equals => if shift { 0x2b } else { 0x3d },        // + / =
        OpenBracket => if shift { 0x7b } else { 0x5b },   // { / [
        CloseBracket => if shift { 0x7d } else { 0x5d },  // } / ]
        Backslash => if shift { 0x7c } else { 0x5c },     // | / backslash
        Semicolon => if shift { 0x3a } else { 0x3b },     // : / ;
        Quote => if shift { 0x22 } else { 0x27 },         // " / '
        Comma => if shift { 0x3c } else { 0x2c },         // < / ,
        Period => if shift { 0x3e } else { 0x2e },        // > / .
        Slash => if shift { 0x3f } else { 0x2f },         // ? / /
        Backtick => if shift { 0x7e } else { 0x60 },      // ~ / `

        Plus => 0x2b,
        Colon => 0x3a,
        Pipe => 0x7c,
        Questionmark => 0x3f,
        Exclamationmark => 0x21,
        OpenCurlyBracket => 0x7b,
        CloseCurlyBracket => 0x7d,

        F1 => 0xffbe,
        F2 => 0xffbf,
        F3 => 0xffc0,
        F4 => 0xffc1,
        F5 => 0xffc2,
        F6 => 0xffc3,
        F7 => 0xffc4,
        F8 => 0xffc5,
        F9 => 0xffc6,
        F10 => 0xffc7,
        F11 => 0xffc8,
        F12 => 0xffc9,
        F13 => 0xffca,
        F14 => 0xffcb,
        F15 => 0xffcc,
        F16 => 0xffcd,
        F17 => 0xffce,
        F18 => 0xffcf,
        F19 => 0xffd0,
        F20 => 0xffd1,

        _ => return None,
    })
}

pub const XK_SHIFT_L: u32 = 0xffe1;
pub const XK_CONTROL_L: u32 = 0xffe3;
pub const XK_ALT_L: u32 = 0xffe9;
