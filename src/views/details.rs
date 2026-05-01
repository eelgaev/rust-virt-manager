use std::sync::Arc;
use egui::{RichText, Ui};

use crate::backend::BackendCommand;
use crate::domain::Guest;
use crate::state::{AppState, HwListItem};
use crate::views::device_editors;

pub fn show_details_tab(ui: &mut Ui, state: &mut AppState, key: &str) {
    let (uri, domain_name) = {
        let vm = &state.vm_windows[key];
        (vm.uri.clone(), vm.domain_name.clone())
    };

    let xml = state
        .connections
        .iter()
        .find(|c| c.uri == uri)
        .and_then(|c| c.domains.get(&domain_name))
        .map(|d| d.xml.clone())
        .unwrap_or_default();

    let guest = Guest::from_xml(&xml).ok();
    let show_xml = state.vm_windows[key].show_xml_editor;

    let hw_items = build_hw_items(guest.as_ref());
    let selected_hw = state.vm_windows[key].selected_hw.clone();

    egui::Panel::left("hw_list_panel")
        .default_size(180.0)
        .max_size(250.0)
        .show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("hw_list")
                .show(ui, |ui| {
                    for (label, item) in &hw_items {
                        if ui.selectable_label(*item == selected_hw, label).clicked() {
                            state.vm_windows.get_mut(key).unwrap().selected_hw = item.clone();
                            state.vm_windows.get_mut(key).unwrap().show_xml_editor = false;
                        }
                    }

                    ui.separator();
                    if ui.button("➕ Add Hardware").clicked() {
                        state.add_hardware.open = true;
                        state.add_hardware.vm_key = key.to_string();
                    }

                    ui.separator();
                    let is_xml = state.vm_windows[key].show_xml_editor;
                    if ui.selectable_label(is_xml, "📝 XML").clicked() {
                        let vm = state.vm_windows.get_mut(key).unwrap();
                        vm.show_xml_editor = !vm.show_xml_editor;
                        if vm.show_xml_editor {
                            vm.xml_editor_text = xml.clone();
                        }
                    }
                });
        });

    egui::ScrollArea::vertical()
        .id_salt("hw_detail")
        .show(ui, |ui| {
            if show_xml {
                show_xml_editor(ui, state, key, &uri);
            } else {
                show_device_panel(ui, state, key, guest.as_ref(), &xml);
            }
        });
}

fn build_hw_items(guest: Option<&Guest>) -> Vec<(String, HwListItem)> {
    let mut items = vec![
        ("📊 Overview".into(), HwListItem::Overview),
        ("📈 Performance".into(), HwListItem::Performance),
        ("🖥 CPUs".into(), HwListItem::Cpu),
        ("💾 Memory".into(), HwListItem::Memory),
        ("🔧 Boot Options".into(), HwListItem::Boot),
    ];

    if let Some(guest) = guest {
        if let Some(devices) = &guest.devices {
            for (i, disk) in devices.disks.iter().enumerate() {
                items.push((format!("💿 {}", disk.display_name()), HwListItem::Disk(i)));
            }
            for (i, nic) in devices.interfaces.iter().enumerate() {
                items.push((format!("🌐 {}", nic.display_name()), HwListItem::Nic(i)));
            }
            for (i, gfx) in devices.graphics.iter().enumerate() {
                items.push((format!("🖵 {}", gfx.display_name()), HwListItem::Graphics(i)));
            }
            for (i, vid) in devices.videos.iter().enumerate() {
                items.push((format!("🎬 {}", vid.display_name()), HwListItem::Video(i)));
            }
            for (i, snd) in devices.sounds.iter().enumerate() {
                items.push((format!("🔊 {}", snd.display_name()), HwListItem::Sound(i)));
            }
            for (i, inp) in devices.inputs.iter().enumerate() {
                items.push((format!("🖱 {}", inp.display_name()), HwListItem::Input(i)));
            }
            for i in 0..devices.serials.len() {
                items.push((format!("📟 Serial {}", i + 1), HwListItem::Char(i)));
            }
            for (i, ctrl) in devices.controllers.iter().enumerate() {
                items.push((format!("🔌 {}", ctrl.display_name()), HwListItem::Controller(i)));
            }
            for (i, hd) in devices.hostdevs.iter().enumerate() {
                items.push((format!("🔗 {}", hd.display_name()), HwListItem::Hostdev(i)));
            }
            for (i, wd) in devices.watchdogs.iter().enumerate() {
                items.push((format!("⏱ {}", wd.display_name()), HwListItem::Watchdog(i)));
            }
            for (i, fs) in devices.filesystems.iter().enumerate() {
                items.push((format!("📁 {}", fs.display_name()), HwListItem::Filesystem(i)));
            }
            for (i, tpm) in devices.tpms.iter().enumerate() {
                items.push((format!("🔐 {}", tpm.display_name()), HwListItem::Tpm(i)));
            }
            for (i, rng) in devices.rngs.iter().enumerate() {
                items.push((format!("🎲 {}", rng.display_name()), HwListItem::Rng(i)));
            }
            for i in 0..devices.vsocks.len() {
                items.push(("🔌 VSOCK".into(), HwListItem::Vsock(i)));
            }
            for (i, rd) in devices.redirdevs.iter().enumerate() {
                items.push((format!("↪ {}", rd.display_name(i)), HwListItem::Redirdev(i)));
            }
            for (i, sc) in devices.smartcards.iter().enumerate() {
                items.push((format!("💳 {}", sc.display_name()), HwListItem::Smartcard(i)));
            }
            for (i, p) in devices.panics.iter().enumerate() {
                items.push((format!("⚠ {}", p.display_name()), HwListItem::Panic(i)));
            }
        }
    }

    items
}

fn show_device_panel(
    ui: &mut Ui,
    state: &mut AppState,
    key: &str,
    guest: Option<&Guest>,
    domain_xml: &str,
) {
    let selected = state.vm_windows[key].selected_hw.clone();

    match &selected {
        HwListItem::Overview => device_editors::show_overview_editor(ui, state, key, guest, domain_xml),
        HwListItem::Performance => show_performance_panel(ui, state, key),
        HwListItem::Cpu => device_editors::show_cpu_editor(ui, state, key, guest, domain_xml),
        HwListItem::Memory => device_editors::show_memory_editor(ui, state, key, guest, domain_xml),
        HwListItem::Boot => device_editors::show_boot_editor(ui, state, key, guest, domain_xml),
        HwListItem::Disk(i) => device_editors::show_disk_editor(ui, state, key, guest, *i, domain_xml),
        HwListItem::Nic(i) => device_editors::show_nic_editor(ui, state, key, guest, *i, domain_xml),
        HwListItem::Graphics(i) => device_editors::show_graphics_editor(ui, state, key, guest, *i, domain_xml),
        HwListItem::Video(i) => device_editors::show_video_editor(ui, state, key, guest, *i, domain_xml),
        HwListItem::Sound(i) => device_editors::show_sound_editor(ui, state, key, guest, *i, domain_xml),
        _ => {
            ui.heading("Device Details");
            ui.separator();
            ui.label("This device type shows read-only information.");
            ui.label("Use the XML editor to modify configuration.");
        }
    }
}

fn show_performance_panel(ui: &mut Ui, state: &AppState, key: &str) {
    ui.heading("Performance");
    ui.separator();

    let vm = &state.vm_windows[key];
    let domain = state
        .connections
        .iter()
        .find(|c| c.uri == vm.uri)
        .and_then(|c| c.domains.get(&vm.domain_name));

    if let Some(domain) = domain {
        let size = egui::vec2(ui.available_width().min(400.0), 80.0);

        ui.label(RichText::new("CPU Usage").strong());
        draw_graph(ui, &domain.stats.cpu, 100.0, egui::Color32::from_rgb(0, 180, 0), size, "CPU %");

        ui.add_space(8.0);
        ui.label(RichText::new("Memory Usage").strong());
        draw_graph(ui, &domain.stats.memory, domain.memory_kib as f64, egui::Color32::from_rgb(0, 100, 200), size, "KiB");
    } else {
        ui.label("No performance data available.");
    }
}

fn draw_graph(
    ui: &mut Ui,
    data: &std::collections::VecDeque<f64>,
    max_val: f64,
    color: egui::Color32,
    size: egui::Vec2,
    label: &str,
) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    if !ui.is_rect_visible(rect) || data.is_empty() {
        return;
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);

    for i in 1..4 {
        let y = rect.top() + rect.height() * (i as f32 / 4.0);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(0.5, ui.visuals().faint_bg_color),
        );
    }

    let max = if max_val > 0.0 { max_val } else { 1.0 };
    let n = data.len();
    if n >= 2 {
        let points: Vec<egui::Pos2> = data
            .iter()
            .enumerate()
            .map(|(i, &val)| {
                let x = rect.left() + (i as f32 / (n - 1) as f32) * rect.width();
                let y = rect.bottom() - (val as f32 / max as f32).clamp(0.0, 1.0) * rect.height();
                egui::pos2(x, y)
            })
            .collect();

        for window in points.windows(2) {
            painter.line_segment([window[0], window[1]], egui::Stroke::new(1.5, color));
        }
    }

    if let Some(&last) = data.back() {
        let text = if max_val == 100.0 {
            format!("{:.1}%", last)
        } else {
            format!("{label}: {:.0}", last)
        };
        painter.text(
            egui::pos2(rect.right() - 4.0, rect.top() + 2.0),
            egui::Align2::RIGHT_TOP,
            text,
            egui::FontId::proportional(10.0),
            color,
        );
    }
}

fn xml_layouter(ui: &Ui, s: &str, wrap_width: f32) -> Arc<egui::Galley> {
    let theme = egui_extras::syntax_highlighting::CodeTheme::from_style(ui.style());
    let mut job =
        egui_extras::syntax_highlighting::highlight(ui.ctx(), ui.style(), &theme, s, "xml");
    job.wrap.max_width = wrap_width;
    ui.ctx().fonts_mut(|f| f.layout_job(job))
}

fn show_xml_editor(ui: &mut Ui, state: &mut AppState, key: &str, uri: &str) {
    ui.heading("XML Editor");
    ui.separator();

    let vm = state.vm_windows.get_mut(key).unwrap();
    let mut layouter = |ui: &Ui, s: &dyn egui::TextBuffer, w: f32| -> Arc<egui::Galley> {
        xml_layouter(ui, s.as_str(), w)
    };
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add(
            egui::TextEdit::multiline(&mut vm.xml_editor_text)
                .code_editor()
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter),
        );
    });

    let xml = state.vm_windows[key].xml_editor_text.clone();
    ui.horizontal(|ui| {
        if ui.button("✅ Apply").clicked() {
            state
                .backend
                .send_to(uri, BackendCommand::DefineXml(uri.to_string(), xml));
        }
        if ui.button("❌ Cancel").clicked() {
            state.vm_windows.get_mut(key).unwrap().show_xml_editor = false;
        }
    });
}
