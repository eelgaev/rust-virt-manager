use egui::{Color32, RichText, TextureOptions, Ui, ViewportBuilder, ViewportId};

use crate::backend::{BackendCommand, DomainAction};
use crate::domain::DomainState;
use crate::state::{AppState, VmTab, VmWindowState};

pub fn show_vm_windows(ctx: &egui::Context, state: &mut AppState) {
    let keys: Vec<String> = state.vm_windows.keys().cloned().collect();
    let mut to_close = Vec::new();

    for key in &keys {
        let title = {
            let vm = &state.vm_windows[key];
            format!("{} - Virtual Machine", vm.domain_name)
        };

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of(format!("vm_win_{key}")),
            ViewportBuilder::default()
                .with_title(&title)
                .with_inner_size([800.0, 600.0]),
            |ui, _class| {
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    to_close.push(key.clone());
                }
                egui::CentralPanel::default().show(ui.ctx(), |ui| {
                    show_vm_window_content(ui, state, key);
                });
            },
        );
    }

    for key in to_close {
        state.vm_windows.remove(&key);
    }
}

fn show_vm_window_content(ui: &mut Ui, state: &mut AppState, key: &str) {
    let (uri, domain_name) = {
        let vm = &state.vm_windows[key];
        (vm.uri.clone(), vm.domain_name.clone())
    };

    let domain_state = state
        .find_connection_mut(&uri)
        .and_then(|c| c.domains.get(&domain_name))
        .map(|d| d.state);

    show_vm_toolbar(ui, state, &uri, &domain_name, domain_state, key);
    ui.separator();

    show_vm_tabs(ui, state, key);
    ui.separator();

    let active_tab = state.vm_windows[key].active_tab;
    match active_tab {
        VmTab::Details => {
            super::details::show_details_tab(ui, state, key);
        }
        VmTab::Console => {
            show_console_tab(ui, state, key, domain_state);
        }
        VmTab::Snapshots => {}
    }
}

fn show_vm_toolbar(
    ui: &mut Ui,
    state: &mut AppState,
    uri: &str,
    domain_name: &str,
    domain_state: Option<DomainState>,
    _key: &str,
) {
    ui.horizontal(|ui| {
        let can_start = domain_state.is_some_and(|s| !s.is_active());
        let can_pause = domain_state.is_some_and(|s| s == DomainState::Running);
        let can_shutdown = domain_state.is_some_and(|s| s.is_active());

        if ui
            .add_enabled(can_start, egui::Button::new("▶ Run"))
            .clicked()
        {
            send_action(state, uri, domain_name, DomainAction::Start);
        }

        if ui
            .add_enabled(can_pause, egui::Button::new("⏸ Pause"))
            .clicked()
        {
            let is_paused = domain_state == Some(DomainState::Paused);
            if is_paused {
                send_action(state, uri, domain_name, DomainAction::Resume);
            } else {
                send_action(state, uri, domain_name, DomainAction::Pause);
            }
        }

        let shutdown_resp = ui.add_enabled(can_shutdown, egui::Button::new("⏻ Shut Down"));
        if shutdown_resp.clicked() {
            send_action(state, uri, domain_name, DomainAction::Shutdown);
        }
        shutdown_resp.context_menu(|ui| {
            if ui.button("🔄 Reboot").clicked() {
                send_action(state, uri, domain_name, DomainAction::Reboot);
                ui.close();
            }
            if ui.button("⏻ Shut Down").clicked() {
                send_action(state, uri, domain_name, DomainAction::Shutdown);
                ui.close();
            }
            if ui.button("⚡ Force Off").clicked() {
                send_action(state, uri, domain_name, DomainAction::ForceOff);
                ui.close();
            }
        });

        ui.separator();

        let state_label = domain_state.map_or("Unknown", |s| s.label());
        let color = match domain_state {
            Some(DomainState::Running) => Color32::from_rgb(0, 180, 0),
            Some(DomainState::Paused) => Color32::from_rgb(200, 180, 0),
            Some(DomainState::Shutoff) => Color32::GRAY,
            _ => Color32::GRAY,
        };
        ui.label(RichText::new(state_label).color(color).strong());
    });
}

fn show_vm_tabs(ui: &mut Ui, state: &mut AppState, key: &str) {
    ui.horizontal(|ui| {
        let vm = state.vm_windows.get_mut(key).unwrap();

        if ui
            .selectable_label(vm.active_tab == VmTab::Console, "🖥 Console")
            .clicked()
        {
            vm.active_tab = VmTab::Console;
        }

        if ui
            .selectable_label(vm.active_tab == VmTab::Details, "📋 Details")
            .clicked()
        {
            vm.active_tab = VmTab::Details;
        }

        // if ui
        //     .selectable_label(vm.active_tab == VmTab::Snapshots, "📸 Snapshots")
        //     .clicked()
        // {
        //     vm.active_tab = VmTab::Snapshots;
        // }
    });
}

fn send_action(state: &mut AppState, uri: &str, name: &str, action: DomainAction) {
    state.backend.send_to(
        uri,
        BackendCommand::DomainAction(uri.to_string(), name.to_string(), action),
    );
}

fn show_console_tab(
    ui: &mut Ui,
    state: &mut AppState,
    key: &str,
    domain_state: Option<DomainState>,
) {
    if !domain_state.is_some_and(|s| s.is_active()) {
        let vm = state.vm_windows.get_mut(key).unwrap();
        if vm.vnc_handle.is_some() {
            vm.vnc_handle = None;
            vm.vnc_texture = None;
        }
        if vm.serial_handle.is_some() {
            vm.serial_handle = None;
        }
        vm.ssh_tunnel = None;
        vm.ssh_tunnel_port = None;
        vm.vnc_retries = 0;
        vm.vnc_retry_after = None;
        ui.centered_and_justified(|ui| {
            ui.label("Guest is not running.");
        });
        return;
    }

    let (vm_uri, vm_name, xml) = {
        let vm = &state.vm_windows[key];
        let xml = state
            .connections
            .iter()
            .find(|c| c.uri == vm.uri)
            .and_then(|c| c.domains.get(&vm.domain_name))
            .map(|d| d.xml.clone())
            .unwrap_or_default();
        (vm.uri.clone(), vm.domain_name.clone(), xml)
    };

    let guest = crate::domain::Guest::from_xml(&xml).ok();
    let vnc_gfx = guest
        .as_ref()
        .and_then(|g| g.devices.as_ref())
        .and_then(|d| d.graphics.iter().find(|g| g.graphics_type == "vnc"));
    let has_vnc = vnc_gfx.is_some();
    let vnc_port = vnc_gfx.and_then(|g| g.port).filter(|&p| p > 0);
    let vnc_password = vnc_gfx.and_then(|g| g.passwd.clone());

    let has_spice = guest
        .as_ref()
        .and_then(|g| g.devices.as_ref())
        .is_some_and(|d| d.graphics.iter().any(|g| g.graphics_type == "spice"));
    let spice_port = guest
        .as_ref()
        .and_then(|g| g.devices.as_ref())
        .and_then(|d| d.graphics.iter().find(|g| g.graphics_type == "spice"))
        .and_then(|g| g.port)
        .filter(|&p| p > 0);

    let is_remote = crate::uri::LibvirtUri::parse(&vm_uri)
        .map(|u| u.is_remote())
        .unwrap_or(false);

    auto_connect_vnc(state, key, has_vnc, vnc_port, vnc_password, &vm_uri, is_remote);

    show_send_key_bar(ui, state, key);
    ui.separator();

    let vm = state.vm_windows.get(key).unwrap();
    let vnc_connected = vm
        .vnc_handle
        .as_ref()
        .is_some_and(|h| h.is_connected());
    let serial_connected = vm
        .serial_handle
        .as_ref()
        .is_some_and(|h| h.is_connected());

    if vnc_connected {
        show_vnc_display(ui, state, key);
    } else if serial_connected {
        show_serial_display(ui, state, key);
    } else {
        let vm = state.vm_windows.get(key).unwrap();
        let vnc_status = vm.vnc_handle.as_ref().map(|h| h.status());
        let serial_status = vm.serial_handle.as_ref().map(|h| h.status());

        match vnc_status {
            Some(crate::console::ConsoleStatus::Connecting) => {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.spinner();
                        ui.label("Connecting to VNC...");
                    });
                });
            }
            Some(crate::console::ConsoleStatus::Error(ref msg)) => {
                let vm = state.vm_windows.get_mut(key).unwrap();
                let retries = vm.vnc_retries;
                if retries < 5 {
                    if vm.vnc_retry_after.is_none() {
                        vm.vnc_retry_after = Some(std::time::Instant::now() + std::time::Duration::from_secs(2));
                        ui.ctx().request_repaint_after(std::time::Duration::from_secs(2));
                    }
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.spinner();
                            ui.label(format!("Connecting to VNC... (attempt {}/5)", retries + 1));
                        });
                    });
                } else {
                    ui.colored_label(Color32::RED, format!("VNC Error: {msg}"));
                    ui.add_space(8.0);
                    if ui.button("🔄 Retry").clicked() {
                        let vm = state.vm_windows.get_mut(key).unwrap();
                        vm.vnc_handle = None;
                        vm.vnc_texture = None;
                        vm.vnc_retry_after = None;
                        vm.vnc_retries = 0;
                        vm.ssh_tunnel = None;
                        vm.ssh_tunnel_port = None;
                    }
                    show_console_fallback(ui, has_vnc, vnc_port, has_spice, spice_port, &vm_uri, &vm_name);
                }
            }
            _ => {
                let has_tunnel = state.vm_windows.get(key).is_some_and(|vm| vm.ssh_tunnel.is_some());
                if has_tunnel {
                    ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.spinner();
                            ui.label("Establishing SSH tunnel...");
                        });
                    });
                } else {
                    match serial_status {
                        Some(crate::console::ConsoleStatus::Connecting) => {
                            ui.centered_and_justified(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.spinner();
                                    ui.label("Connecting to serial console...");
                                });
                            });
                        }
                        Some(crate::console::ConsoleStatus::Error(ref msg)) => {
                            ui.colored_label(Color32::RED, format!("Serial Error: {msg}"));
                        }
                        _ => {
                            show_console_fallback(ui, has_vnc, vnc_port, has_spice, spice_port, &vm_uri, &vm_name);
                        }
                    }
                }
            },
        }
    }
}

fn auto_connect_vnc(
    state: &mut AppState,
    key: &str,
    has_vnc: bool,
    vnc_port: Option<i32>,
    vnc_password: Option<String>,
    uri: &str,
    is_remote: bool,
) {
    let vm = state.vm_windows.get(key).unwrap();
    if vm.serial_handle.is_some() {
        return;
    }

    if let Some(handle) = &vm.vnc_handle {
        if handle.is_connected() || matches!(handle.status(), crate::console::ConsoleStatus::Connecting) {
            return;
        }
        let retry_after = vm.vnc_retry_after;
        let retries = vm.vnc_retries;
        if retries >= 5 {
            return;
        }
        if let Some(t) = retry_after {
            if std::time::Instant::now() < t {
                return;
            }
        }
        let vm = state.vm_windows.get_mut(key).unwrap();
        vm.vnc_handle = None;
        vm.vnc_texture = None;
        vm.vnc_retry_after = None;
        vm.vnc_retries = retries + 1;
    }

    if !has_vnc {
        return;
    }
    let port = match vnc_port {
        Some(p) => p,
        None => return,
    };

    let (host, actual_port) = if is_remote {
        let vm = state.vm_windows.get_mut(key).unwrap();
        if let Some(ref mut tunnel) = vm.ssh_tunnel {
            if !tunnel.is_alive() {
                vm.ssh_tunnel = None;
                vm.ssh_tunnel_port = None;
                state.set_error("SSH tunnel process died".into());
                return;
            }
            let lp = vm.ssh_tunnel_port.unwrap();
            if !tunnel.is_ready() {
                vm.vnc_retry_after = Some(std::time::Instant::now() + std::time::Duration::from_secs(1));
                return;
            }
            ("127.0.0.1".to_string(), lp)
        } else {
            let parsed = crate::uri::LibvirtUri::parse(uri).ok();
            if let Some(parsed) = &parsed {
                if !parsed.hostname.is_empty() {
                    match crate::console::ssh_tunnel::SshTunnel::new(
                        parsed.username.as_deref(),
                        &parsed.hostname,
                        port as u16,
                    ) {
                        Ok(tunnel) => {
                            let lp = tunnel.local_port();
                            let vm = state.vm_windows.get_mut(key).unwrap();
                            vm.ssh_tunnel = Some(tunnel);
                            vm.ssh_tunnel_port = Some(lp);
                            vm.vnc_retry_after = Some(std::time::Instant::now() + std::time::Duration::from_secs(1));
                            return;
                        }
                        Err(e) => {
                            state.set_error(format!("SSH tunnel failed: {e}"));
                            return;
                        }
                    }
                } else {
                    ("127.0.0.1".to_string(), port as u16)
                }
            } else {
                ("127.0.0.1".to_string(), port as u16)
            }
        }
    } else {
        ("127.0.0.1".to_string(), port as u16)
    };

    let handle = crate::console::vnc::VncHandle::connect(&host, actual_port, vnc_password);
    state.vm_windows.get_mut(key).unwrap().vnc_handle = Some(handle);
}

fn send_vnc_key_combo(state: &AppState, key: &str, keysyms: &[(u32, bool)]) {
    if let Some(handle) = state.vm_windows.get(key).and_then(|vm| vm.vnc_handle.as_ref()) {
        for &(keysym, pressed) in keysyms {
            handle.send_key(keysym, pressed);
        }
    }
}

fn show_send_key_bar(ui: &mut Ui, state: &mut AppState, key: &str) {
    let vnc_active = state.vm_windows.get(key)
        .and_then(|vm| vm.vnc_handle.as_ref())
        .is_some_and(|h| h.is_connected());

    ui.horizontal(|ui| {
        ui.add_enabled_ui(vnc_active, |ui| {
            ui.menu_button("Send Key", |ui| {
                if ui.button("Ctrl+Alt+Del").clicked() {
                    send_vnc_key_combo(state, key, &[
                        (0xffe3, true), (0xffe9, true), (0xffff, true),
                        (0xffff, false), (0xffe9, false), (0xffe3, false),
                    ]);
                    ui.close();
                }
                if ui.button("Ctrl+Alt+Backspace").clicked() {
                    send_vnc_key_combo(state, key, &[
                        (0xffe3, true), (0xffe9, true), (0xff08, true),
                        (0xff08, false), (0xffe9, false), (0xffe3, false),
                    ]);
                    ui.close();
                }
                ui.separator();
                for fkey in 1..=12 {
                    let keysym = 0xffbe + (fkey - 1) as u32;
                    if ui.button(format!("Ctrl+Alt+F{fkey}")).clicked() {
                        send_vnc_key_combo(state, key, &[
                            (0xffe3, true), (0xffe9, true), (keysym, true),
                            (keysym, false), (0xffe9, false), (0xffe3, false),
                        ]);
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("PrintScreen").clicked() {
                    send_vnc_key_combo(state, key, &[(0xff61, true), (0xff61, false)]);
                    ui.close();
                }
            });
        });

        ui.separator();

        let vm = state.vm_windows.get(key).unwrap();
        let serial_active = vm.serial_handle.is_some();
        let vm_uri = vm.uri.clone();
        let vm_name = vm.domain_name.clone();

        if serial_active {
            if ui.button("Switch to graphical console").clicked() {
                state.vm_windows.get_mut(key).unwrap().serial_handle = None;
            }
        } else if ui.button("Serial Console").clicked() {
            let vm = state.vm_windows.get_mut(key).unwrap();
            vm.vnc_handle = None;
            vm.vnc_texture = None;
            let handle = crate::console::serial::SerialHandle::connect(&vm_uri, &vm_name);
            state.vm_windows.get_mut(key).unwrap().serial_handle = Some(handle);
        }
    });
}

fn show_vnc_display(ui: &mut Ui, state: &mut AppState, key: &str) {
    let vm = state.vm_windows.get_mut(key).unwrap();

    if let Some(handle) = &vm.vnc_handle {
        if let Some(image) = handle.take_framebuffer_if_dirty() {
            match &mut vm.vnc_texture {
                Some(tex) => tex.set(image, TextureOptions::NEAREST),
                None => {
                    vm.vnc_texture =
                        Some(ui.ctx().load_texture("vnc_fb", image, TextureOptions::NEAREST));
                }
            }
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    }

    let fb_size = vm.vnc_handle.as_ref().and_then(|h| h.framebuffer_size());
    let tex_size = vm.vnc_texture.as_ref().map(|t| t.size_vec2());
    let tex_id = vm.vnc_texture.as_ref().map(|t| t.id());

    let (fb_w, fb_h) = match fb_size {
        Some([w, h]) => (w as f32, h as f32),
        None => return,
    };
    let (Some(size), Some(id)) = (tex_size, tex_id) else {
        return;
    };

    let available = ui.available_size();
    let scale = (available.x / size.x).min(available.y / size.y).min(1.0);
    let display_size = size * scale;

    let offset = egui::vec2(
        (available.x - display_size.x) * 0.5,
        (available.y - display_size.y) * 0.5,
    );

    let full_rect = ui.available_rect_before_wrap();
    let image_rect = egui::Rect::from_min_size(full_rect.min + offset, display_size);

    let response = ui.allocate_rect(full_rect, egui::Sense::click_and_drag());

    if ui.is_rect_visible(image_rect) {
        ui.painter().image(
            id,
            image_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    if response.hovered() || response.has_focus() {
        response.request_focus();
        ui.ctx().request_repaint();

        if let Some(handle) = &state.vm_windows[key].vnc_handle {
            let pos = response
                .interact_pointer_pos()
                .or_else(|| response.hover_pos());

            if let Some(pos) = pos {
                let rel = pos - image_rect.min;
                let raw_x = rel.x / scale;
                let raw_y = rel.y / scale;
                let x = raw_x.clamp(0.0, fb_w - 1.0) as u16;
                let y = raw_y.clamp(0.0, fb_h - 1.0) as u16;

                let mut buttons: u8 = 0;
                if ui.input(|i| i.pointer.primary_down()) {
                    buttons |= 1;
                }
                if ui.input(|i| i.pointer.middle_down()) {
                    buttons |= 2;
                }
                if ui.input(|i| i.pointer.secondary_down()) {
                    buttons |= 4;
                }

                let scroll = ui.input(|i| i.smooth_scroll_delta);
                if scroll.y > 0.0 {
                    handle.send_mouse(x, y, buttons | 8);
                    handle.send_mouse(x, y, buttons);
                } else if scroll.y < 0.0 {
                    handle.send_mouse(x, y, buttons | 16);
                    handle.send_mouse(x, y, buttons);
                } else {
                    handle.send_mouse(x, y, buttons);
                }
            }

            let events: Vec<egui::Event> = ui.input(|i| i.events.clone());
            for event in &events {
                if let egui::Event::Key {
                    key, pressed, modifiers, ..
                } = event
                {
                    if let Some(keysym) = crate::console::vnc::key_to_keysym(
                        *key,
                        modifiers.shift,
                    ) {
                        if *pressed {
                            if modifiers.ctrl {
                                handle.send_key(crate::console::vnc::XK_CONTROL_L, true);
                            }
                            if modifiers.alt {
                                handle.send_key(crate::console::vnc::XK_ALT_L, true);
                            }
                            handle.send_key(keysym, true);
                        } else {
                            handle.send_key(keysym, false);
                            if modifiers.alt {
                                handle.send_key(crate::console::vnc::XK_ALT_L, false);
                            }
                            if modifiers.ctrl {
                                handle.send_key(crate::console::vnc::XK_CONTROL_L, false);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn show_serial_display(ui: &mut Ui, state: &mut AppState, key: &str) {
    let vm = state.vm_windows.get(key).unwrap();
    let handle = match &vm.serial_handle {
        Some(h) => h,
        None => return,
    };

    let _ = handle.take_screen_if_dirty();
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));

    let screen = handle.screen();

    let font_id = egui::FontId::monospace(14.0);
    let row_height = 16.0_f32;

    egui::ScrollArea::vertical()
        .id_salt("serial_scroll")
        .show(ui, |ui| {
            let (response, painter) = ui.allocate_painter(
                egui::vec2(
                    screen.cols as f32 * 8.4,
                    screen.rows as f32 * row_height,
                ),
                egui::Sense::click(),
            );

            if response.clicked() {
                response.request_focus();
            }

            let origin = response.rect.min;

            for (row_idx, row) in screen.cells.iter().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    let x = origin.x + col_idx as f32 * 8.4;
                    let y = origin.y + row_idx as f32 * row_height;

                    if cell.bg != Color32::BLACK {
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(x, y),
                                egui::vec2(8.4, row_height),
                            ),
                            0.0,
                            cell.bg,
                        );
                    }

                    let color = if cell.bold {
                        brighten(cell.fg)
                    } else {
                        cell.fg
                    };

                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::LEFT_TOP,
                        &cell.ch,
                        font_id.clone(),
                        color,
                    );
                }
            }

            let cx = origin.x + screen.cursor.1 as f32 * 8.4;
            let cy = origin.y + screen.cursor.0 as f32 * row_height;
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(cx, cy + row_height - 2.0),
                    egui::vec2(8.4, 2.0),
                ),
                0.0,
                Color32::LIGHT_GRAY,
            );

            if response.has_focus() {
                let events: Vec<egui::Event> = ui.input(|i| i.events.clone());
                let handle = state.vm_windows[key].serial_handle.as_ref().unwrap();
                for event in &events {
                    match event {
                        egui::Event::Text(text) => {
                            handle.send_input(text.as_bytes());
                        }
                        egui::Event::Key {
                            key, pressed: true, ..
                        } => {
                            handle.send_key(*key);
                        }
                        _ => {}
                    }
                }
            }
        });
}

fn show_console_fallback(
    ui: &mut Ui,
    has_vnc: bool,
    vnc_port: Option<i32>,
    has_spice: bool,
    spice_port: Option<i32>,
    uri: &str,
    domain_name: &str,
) {
    ui.add_space(16.0);

    if has_spice {
        ui.vertical_centered(|ui| {
            let port_label = spice_port.map_or("auto".into(), |p: i32| format!("{p}"));
            ui.label(format!("SPICE display on port {port_label}"));
            ui.add_space(8.0);
            if ui.button("🖥 Open Graphical Console (virt-viewer)").clicked() {
                let _ = std::process::Command::new("virt-viewer")
                    .args(["--connect", uri, domain_name])
                    .spawn();
            }
            if let Some(port) = spice_port {
                if ui.button("🖥 Open with remote-viewer").clicked() {
                    let _ = std::process::Command::new("remote-viewer")
                        .arg(format!("spice://localhost:{port}"))
                        .spawn();
                }
            }
        });
    }

    if has_vnc {
        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            let port_label = vnc_port.map_or("auto".into(), |p: i32| format!("{p}"));
            ui.label(format!("VNC display on port {port_label}"));
            ui.add_space(4.0);
            if ui.button("🖥 Open with virt-viewer (VNC)").clicked() {
                let _ = std::process::Command::new("virt-viewer")
                    .args(["--connect", uri, domain_name])
                    .spawn();
            }
            if let Some(port) = vnc_port {
                if ui.button("🖥 Open with remote-viewer").clicked() {
                    let _ = std::process::Command::new("remote-viewer")
                        .arg(format!("vnc://localhost:{port}"))
                        .spawn();
                }
            }
        });
    }

    if !has_vnc && !has_spice {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.label("No graphical display configured for this VM.");
            ui.label("Add a VNC or SPICE graphics device in the Details tab.");
        });
    }
}

fn brighten(color: Color32) -> Color32 {
    let [r, g, b, a] = color.to_array();
    Color32::from_rgba_premultiplied(
        r.saturating_add(60),
        g.saturating_add(60),
        b.saturating_add(60),
        a,
    )
}

pub fn open_vm_window(state: &mut AppState, uri: &str, domain_name: &str) {
    let key = format!("{uri}+{domain_name}");
    if !state.vm_windows.contains_key(&key) {
        state
            .vm_windows
            .insert(key, VmWindowState::new(uri.to_string(), domain_name.to_string()));
    }
}
