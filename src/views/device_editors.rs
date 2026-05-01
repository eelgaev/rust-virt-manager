use std::sync::Arc;
use egui::{RichText, Ui};

use crate::backend::BackendCommand;
use crate::domain::Guest;
use crate::edit_state::*;
use crate::state::AppState;
use crate::xml_helpers;

fn xml_layouter(ui: &Ui, s: &str, wrap_width: f32) -> Arc<egui::Galley> {
    let theme = egui_extras::syntax_highlighting::CodeTheme::from_style(ui.style());
    let mut job =
        egui_extras::syntax_highlighting::highlight(ui.ctx(), ui.style(), &theme, s, "xml");
    job.wrap.max_width = wrap_width;
    ui.ctx().fonts_mut(|f| f.layout_job(job))
}

fn show_sub_tabs(ui: &mut Ui, sub_tab: &mut DeviceSubTab) {
    ui.horizontal(|ui| {
        if ui
            .selectable_label(*sub_tab == DeviceSubTab::Details, "Details")
            .clicked()
        {
            *sub_tab = DeviceSubTab::Details;
        }
        if ui
            .selectable_label(*sub_tab == DeviceSubTab::Xml, "XML")
            .clicked()
        {
            *sub_tab = DeviceSubTab::Xml;
        }
    });
    ui.separator();
}

fn show_device_xml_editor(ui: &mut Ui, xml_text: &mut String, dirty: &mut bool) {
    let mut layouter = |ui: &Ui, s: &dyn egui::TextBuffer, w: f32| -> Arc<egui::Galley> { xml_layouter(ui, s.as_str(), w) };
    let response = ui.add(
        egui::TextEdit::multiline(xml_text)
            .code_editor()
            .desired_width(f32::INFINITY)
            .layouter(&mut layouter),
    );
    if response.changed() {
        *dirty = true;
    }
}

fn show_apply_revert(
    ui: &mut Ui,
    is_dirty: bool,
    is_running: bool,
) -> (bool, bool) {
    let mut apply = false;
    let mut revert = false;
    ui.separator();
    ui.horizontal(|ui| {
        if ui.add_enabled(is_dirty, egui::Button::new("✅ Apply")).clicked() {
            apply = true;
        }
        if ui.add_enabled(is_dirty, egui::Button::new("↩ Revert")).clicked() {
            revert = true;
        }
        if is_running && is_dirty {
            ui.label(
                RichText::new("⚠ VM is running. Some changes need shutdown.")
                    .small()
                    .color(egui::Color32::YELLOW),
            );
        }
    });
    (apply, revert)
}

fn get_domain_running(state: &AppState, key: &str) -> bool {
    let vm = &state.vm_windows[key];
    state
        .connections
        .iter()
        .find(|c| c.uri == vm.uri)
        .and_then(|c| c.domains.get(&vm.domain_name))
        .is_some_and(|d| d.state.is_active())
}

fn get_qemu_caps(state: &AppState, key: &str) -> crate::qemu_capabilities::QemuCapabilities {
    let vm = &state.vm_windows[key];
    let xml = state
        .connections
        .iter()
        .find(|c| c.uri == vm.uri)
        .and_then(|c| c.domains.get(&vm.domain_name))
        .map(|d| d.xml.clone())
        .unwrap_or_default();
    let arch = Guest::from_xml(&xml)
        .ok()
        .and_then(|g| g.os.os_type.arch.clone())
        .unwrap_or_else(|| "x86_64".into());
    state
        .qemu_caps
        .get(&arch)
        .cloned()
        .unwrap_or_else(crate::qemu_capabilities::QemuCapabilities::fallback)
}

fn send_define(state: &mut AppState, key: &str, new_xml: String) {
    let uri = state.vm_windows[key].uri.clone();
    state
        .backend
        .send_to(&uri, BackendCommand::DefineXml(uri.clone(), new_xml));
}

fn combo_box(ui: &mut Ui, id: &str, current: &mut String, options: &[String], dirty: &mut bool) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(current.as_str())
        .show_ui(ui, |ui| {
            for opt in options {
                if ui.selectable_label(*current == *opt, opt).clicked() {
                    *current = opt.clone();
                    *dirty = true;
                }
            }
        });
}

pub fn show_overview_editor(
    ui: &mut Ui,
    state: &mut AppState,
    key: &str,
    guest: Option<&Guest>,
    domain_xml: &str,
) {
    let guest = match guest {
        Some(g) => g,
        None => {
            ui.label("No domain XML available.");
            return;
        }
    };

    let edit = state.vm_windows.get_mut(key).unwrap();
    if edit.device_edits.overview.is_none() {
        edit.device_edits.overview = Some(DeviceEditState::new(OverviewEdit::from_guest(guest)));
    }
    let edit_state = edit.device_edits.overview.as_mut().unwrap();

    show_sub_tabs(ui, &mut edit_state.sub_tab);

    match edit_state.sub_tab {
        DeviceSubTab::Details => {
            ui.heading("Overview");
            ui.separator();
            egui::Grid::new("overview_edit_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Name:");
                    if ui
                        .text_edit_singleline(&mut edit_state.fields.name)
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Title:");
                    if ui
                        .text_edit_singleline(&mut edit_state.fields.title)
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Description:");
                    if ui
                        .text_edit_singleline(&mut edit_state.fields.description)
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();

                    ui.label("UUID:");
                    ui.label(guest.uuid.as_deref().unwrap_or("N/A"));
                    ui.end_row();

                    ui.label("Type:");
                    ui.label(&guest.domain_type);
                    ui.end_row();

                    ui.label("Architecture:");
                    ui.label(guest.os.os_type.arch.as_deref().unwrap_or("N/A"));
                    ui.end_row();

                    ui.label("Machine:");
                    ui.label(guest.os.os_type.machine.as_deref().unwrap_or("N/A"));
                    ui.end_row();

                    ui.label("Firmware:");
                    if let Some(loader) = &guest.os.loader {
                        ui.label(loader.path.as_deref().unwrap_or("UEFI"));
                    } else {
                        ui.label("BIOS");
                    }
                    ui.end_row();
                });
        }
        DeviceSubTab::Xml => {
            if edit_state.device_xml_text.is_empty() {
                edit_state.device_xml_text = domain_xml.to_string();
            }
            show_device_xml_editor(
                ui,
                &mut edit_state.device_xml_text,
                &mut edit_state.device_xml_dirty,
            );
        }
    }

    let is_running = get_domain_running(state, key);
    let edit_state = state.vm_windows[key].device_edits.overview.as_ref().unwrap();
    let (apply, revert) = show_apply_revert(ui, edit_state.is_dirty(), is_running);

    if apply {
        let edit_state = state.vm_windows[key].device_edits.overview.as_ref().unwrap();
        if edit_state.sub_tab == DeviceSubTab::Xml {
            let new_xml = edit_state.device_xml_text.clone();
            send_define(state, key, new_xml);
        } else {
            let parts = edit_state.fields.to_xml_parts();
            let mut xml = domain_xml.to_string();
            for (tag, new_elem) in &parts {
                if let Some(replaced) = xml_helpers::replace_toplevel_element(&xml, tag, new_elem) {
                    xml = replaced;
                }
            }
            send_define(state, key, xml);
        }
        let edit_state = state.vm_windows.get_mut(key).unwrap().device_edits.overview.as_mut().unwrap();
        edit_state.dirty = false;
        edit_state.device_xml_dirty = false;
    }
    if revert {
        state.vm_windows.get_mut(key).unwrap().device_edits.overview = None;
    }
}

pub fn show_cpu_editor(
    ui: &mut Ui,
    state: &mut AppState,
    key: &str,
    guest: Option<&Guest>,
    domain_xml: &str,
) {
    let guest = match guest {
        Some(g) => g,
        None => return,
    };

    let edit = state.vm_windows.get_mut(key).unwrap();
    if edit.device_edits.cpu.is_none() {
        edit.device_edits.cpu = Some(DeviceEditState::new(CpuEdit::from_guest(guest)));
    }
    let edit_state = edit.device_edits.cpu.as_mut().unwrap();

    show_sub_tabs(ui, &mut edit_state.sub_tab);

    match edit_state.sub_tab {
        DeviceSubTab::Details => {
            ui.heading("CPUs");
            ui.separator();
            egui::Grid::new("cpu_edit_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("vCPUs (max):");
                    if ui
                        .add(egui::DragValue::new(&mut edit_state.fields.vcpu_count).range(1..=256))
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Current vCPUs:");
                    ui.horizontal(|ui| {
                        if ui
                            .checkbox(&mut edit_state.fields.has_current, "")
                            .changed()
                        {
                            edit_state.dirty = true;
                        }
                        if edit_state.fields.has_current {
                            if ui
                                .add(
                                    egui::DragValue::new(&mut edit_state.fields.current)
                                        .range(1..=edit_state.fields.vcpu_count),
                                )
                                .changed()
                            {
                                edit_state.dirty = true;
                            }
                        }
                    });
                    ui.end_row();
                });
        }
        DeviceSubTab::Xml => {
            if edit_state.device_xml_text.is_empty() {
                edit_state.device_xml_text = xml_helpers::find_toplevel_element(domain_xml, "vcpu")
                    .map(|s| s.content)
                    .unwrap_or_default();
            }
            show_device_xml_editor(
                ui,
                &mut edit_state.device_xml_text,
                &mut edit_state.device_xml_dirty,
            );
        }
    }

    let is_running = get_domain_running(state, key);
    let edit_state = state.vm_windows[key].device_edits.cpu.as_ref().unwrap();
    let (apply, revert) = show_apply_revert(ui, edit_state.is_dirty(), is_running);

    if apply {
        let edit_state = state.vm_windows[key].device_edits.cpu.as_ref().unwrap();
        if edit_state.sub_tab == DeviceSubTab::Xml {
            let snippet = edit_state.device_xml_text.clone();
            if let Some(new_xml) = xml_helpers::replace_toplevel_element(domain_xml, "vcpu", &snippet) {
                send_define(state, key, new_xml);
            }
        } else {
            let vcpu_xml = edit_state.fields.to_xml();
            if let Some(new_xml) = xml_helpers::replace_toplevel_element(domain_xml, "vcpu", &vcpu_xml) {
                send_define(state, key, new_xml);
            }
        }
        let e = state.vm_windows.get_mut(key).unwrap().device_edits.cpu.as_mut().unwrap();
        e.dirty = false;
        e.device_xml_dirty = false;
    }
    if revert {
        state.vm_windows.get_mut(key).unwrap().device_edits.cpu = None;
    }
}

pub fn show_memory_editor(
    ui: &mut Ui,
    state: &mut AppState,
    key: &str,
    guest: Option<&Guest>,
    domain_xml: &str,
) {
    let guest = match guest {
        Some(g) => g,
        None => return,
    };

    let edit = state.vm_windows.get_mut(key).unwrap();
    if edit.device_edits.memory.is_none() {
        edit.device_edits.memory = Some(DeviceEditState::new(MemoryEdit::from_guest(guest)));
    }
    let edit_state = edit.device_edits.memory.as_mut().unwrap();

    show_sub_tabs(ui, &mut edit_state.sub_tab);

    match edit_state.sub_tab {
        DeviceSubTab::Details => {
            ui.heading("Memory");
            ui.separator();
            egui::Grid::new("mem_edit_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Maximum (MiB):");
                    if ui
                        .add(egui::DragValue::new(&mut edit_state.fields.max_memory_mib).range(64..=4194304))
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Current (MiB):");
                    if ui
                        .add(
                            egui::DragValue::new(&mut edit_state.fields.current_memory_mib)
                                .range(64..=edit_state.fields.max_memory_mib),
                        )
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();
                });
        }
        DeviceSubTab::Xml => {
            if edit_state.device_xml_text.is_empty() {
                let mem = xml_helpers::find_toplevel_element(domain_xml, "memory")
                    .map(|s| s.content)
                    .unwrap_or_default();
                let cur = xml_helpers::find_toplevel_element(domain_xml, "currentMemory")
                    .map(|s| s.content)
                    .unwrap_or_default();
                edit_state.device_xml_text = format!("{mem}\n{cur}");
            }
            show_device_xml_editor(
                ui,
                &mut edit_state.device_xml_text,
                &mut edit_state.device_xml_dirty,
            );
        }
    }

    let is_running = get_domain_running(state, key);
    let edit_state = state.vm_windows[key].device_edits.memory.as_ref().unwrap();
    let (apply, revert) = show_apply_revert(ui, edit_state.is_dirty(), is_running);

    if apply {
        let edit_state = state.vm_windows[key].device_edits.memory.as_ref().unwrap();
        let (mem_xml, cur_xml) = edit_state.fields.to_xml();
        let mut xml = domain_xml.to_string();
        if let Some(r) = xml_helpers::replace_toplevel_element(&xml, "memory", &mem_xml) {
            xml = r;
        }
        if let Some(r) = xml_helpers::replace_toplevel_element(&xml, "currentMemory", &cur_xml) {
            xml = r;
        }
        send_define(state, key, xml);
        let e = state.vm_windows.get_mut(key).unwrap().device_edits.memory.as_mut().unwrap();
        e.dirty = false;
        e.device_xml_dirty = false;
    }
    if revert {
        state.vm_windows.get_mut(key).unwrap().device_edits.memory = None;
    }
}

pub fn show_boot_editor(
    ui: &mut Ui,
    state: &mut AppState,
    key: &str,
    guest: Option<&Guest>,
    domain_xml: &str,
) {
    let guest = match guest {
        Some(g) => g,
        None => return,
    };

    let edit = state.vm_windows.get_mut(key).unwrap();
    if edit.device_edits.boot.is_none() {
        edit.device_edits.boot = Some(DeviceEditState::new(BootEdit::from_guest(guest)));
    }
    let edit_state = edit.device_edits.boot.as_mut().unwrap();

    show_sub_tabs(ui, &mut edit_state.sub_tab);

    match edit_state.sub_tab {
        DeviceSubTab::Details => {
            ui.heading("Boot Options");
            ui.separator();

            let available = ["hd", "cdrom", "network", "fd"];
            let mut to_remove = None;
            let mut to_swap = None;

            for (i, dev) in edit_state.fields.boot_devices.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}. {dev}", i + 1));
                    if i > 0 && ui.small_button("⬆").clicked() {
                        to_swap = Some((i, i - 1));
                    }
                    if i + 1 < edit_state.fields.boot_devices.len()
                        && ui.small_button("⬇").clicked()
                    {
                        to_swap = Some((i, i + 1));
                    }
                    if ui.small_button("✕").clicked() {
                        to_remove = Some(i);
                    }
                });
            }

            if let Some((a, b)) = to_swap {
                edit_state.fields.boot_devices.swap(a, b);
                edit_state.dirty = true;
            }
            if let Some(i) = to_remove {
                edit_state.fields.boot_devices.remove(i);
                edit_state.dirty = true;
            }

            let existing: Vec<String> = edit_state.fields.boot_devices.clone();
            let addable: Vec<&&str> = available.iter().filter(|d| !existing.contains(&d.to_string())).collect();
            if !addable.is_empty() {
                ui.add_space(4.0);
                ui.menu_button("➕ Add boot device", |ui| {
                    for dev in &addable {
                        if ui.button(**dev).clicked() {
                            edit_state.fields.boot_devices.push(dev.to_string());
                            edit_state.dirty = true;
                            ui.close();
                        }
                    }
                });
            }

            if let Some(loader) = &guest.os.loader {
                ui.add_space(8.0);
                ui.label(RichText::new("Firmware").strong());
                if let Some(path) = &loader.path {
                    ui.label(format!("Path: {path}"));
                }
                if let Some(lt) = &loader.loader_type {
                    ui.label(format!("Type: {lt}"));
                }
            }
        }
        DeviceSubTab::Xml => {
            if edit_state.device_xml_text.is_empty() {
                edit_state.device_xml_text = xml_helpers::find_toplevel_element(domain_xml, "os")
                    .map(|s| s.content)
                    .unwrap_or_default();
            }
            show_device_xml_editor(
                ui,
                &mut edit_state.device_xml_text,
                &mut edit_state.device_xml_dirty,
            );
        }
    }

    let is_running = get_domain_running(state, key);
    let edit_state = state.vm_windows[key].device_edits.boot.as_ref().unwrap();
    let (apply, revert) = show_apply_revert(ui, edit_state.is_dirty(), is_running);

    if apply {
        let edit_state = state.vm_windows[key].device_edits.boot.as_ref().unwrap();
        if edit_state.sub_tab == DeviceSubTab::Xml {
            let snippet = edit_state.device_xml_text.clone();
            if let Some(new_xml) = xml_helpers::replace_toplevel_element(domain_xml, "os", &snippet) {
                send_define(state, key, new_xml);
            }
        } else {
            let os_xml = xml_helpers::find_toplevel_element(domain_xml, "os")
                .map(|s| s.content)
                .unwrap_or_default();
            let mut new_os = String::new();
            if let Some(type_end) = os_xml.find("</type>") {
                let after_type = type_end + 7;
                new_os.push_str(&os_xml[..after_type]);
            } else {
                new_os.push_str("<os>");
            }
            for dev in &edit_state.fields.boot_devices {
                new_os.push_str(&format!("\n    <boot dev='{dev}'/>"));
            }
            if let Some(loader) = &guest.os.loader {
                new_os.push_str("\n    <loader");
                if let Some(lt) = &loader.loader_type {
                    new_os.push_str(&format!(" type='{lt}'"));
                }
                if let Some(ro) = &loader.readonly {
                    new_os.push_str(&format!(" readonly='{ro}'"));
                }
                if let Some(sec) = &loader.secure {
                    new_os.push_str(&format!(" secure='{sec}'"));
                }
                if let Some(path) = &loader.path {
                    new_os.push_str(&format!(">{path}</loader>"));
                } else {
                    new_os.push_str("/>");
                }
            }
            new_os.push_str("\n  </os>");
            if let Some(new_xml) = xml_helpers::replace_toplevel_element(domain_xml, "os", &new_os) {
                send_define(state, key, new_xml);
            }
        }
        let e = state.vm_windows.get_mut(key).unwrap().device_edits.boot.as_mut().unwrap();
        e.dirty = false;
        e.device_xml_dirty = false;
    }
    if revert {
        state.vm_windows.get_mut(key).unwrap().device_edits.boot = None;
    }
}

pub fn show_disk_editor(
    ui: &mut Ui,
    state: &mut AppState,
    key: &str,
    guest: Option<&Guest>,
    idx: usize,
    domain_xml: &str,
) {
    let guest = match guest {
        Some(g) => g,
        None => return,
    };

    let caps = get_qemu_caps(state, key);
    let edit = state.vm_windows.get_mut(key).unwrap();
    if !edit.device_edits.disk.contains_key(&idx) {
        if let Some(de) = DiskEdit::from_guest(guest, idx) {
            edit.device_edits.disk.insert(idx, DeviceEditState::new(de));
        } else {
            return;
        }
    }
    let edit_state = edit.device_edits.disk.get_mut(&idx).unwrap();

    show_sub_tabs(ui, &mut edit_state.sub_tab);

    match edit_state.sub_tab {
        DeviceSubTab::Details => {
            ui.heading("Disk");
            ui.separator();
            egui::Grid::new(format!("disk_edit_{idx}"))
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Source:");
                    ui.horizontal(|ui| {
                        if ui
                            .add(egui::TextEdit::singleline(&mut edit_state.fields.source_path).desired_width(250.0))
                            .changed()
                        {
                            edit_state.dirty = true;
                        }
                        if ui.button("📁").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Disk Images", &["qcow2", "raw", "img", "vmdk", "iso"])
                                .pick_file()
                            {
                                edit_state.fields.source_path = path.display().to_string();
                                edit_state.dirty = true;
                            }
                        }
                    });
                    ui.end_row();

                    ui.label("Target:");
                    if ui
                        .text_edit_singleline(&mut edit_state.fields.target_dev)
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Bus:");
                    combo_box(
                        ui,
                        &format!("disk_bus_{idx}"),
                        &mut edit_state.fields.bus,
                        &caps.disk_buses,
                        &mut edit_state.dirty,
                    );
                    ui.end_row();

                    ui.label("Device:");
                    let device_opts = vec!["disk".into(), "cdrom".into(), "floppy".into()];
                    combo_box(
                        ui,
                        &format!("disk_dev_{idx}"),
                        &mut edit_state.fields.device_type,
                        &device_opts,
                        &mut edit_state.dirty,
                    );
                    ui.end_row();

                    ui.label("Format:");
                    let fmt_opts = vec!["qcow2".into(), "raw".into(), "vmdk".into(), "vpc".into()];
                    combo_box(
                        ui,
                        &format!("disk_fmt_{idx}"),
                        &mut edit_state.fields.format,
                        &fmt_opts,
                        &mut edit_state.dirty,
                    );
                    ui.end_row();

                    ui.label("Cache:");
                    let cache_opts = vec![
                        String::new(),
                        "none".into(),
                        "writeback".into(),
                        "writethrough".into(),
                        "directsync".into(),
                        "unsafe".into(),
                    ];
                    combo_box(
                        ui,
                        &format!("disk_cache_{idx}"),
                        &mut edit_state.fields.cache,
                        &cache_opts,
                        &mut edit_state.dirty,
                    );
                    ui.end_row();

                    ui.label("Read-only:");
                    if ui.checkbox(&mut edit_state.fields.readonly, "").changed() {
                        edit_state.dirty = true;
                    }
                    ui.end_row();
                });
        }
        DeviceSubTab::Xml => {
            if edit_state.device_xml_text.is_empty() {
                edit_state.device_xml_text =
                    xml_helpers::find_nth_device_element(domain_xml, "disk", idx)
                        .map(|s| s.content)
                        .unwrap_or_default();
            }
            show_device_xml_editor(
                ui,
                &mut edit_state.device_xml_text,
                &mut edit_state.device_xml_dirty,
            );
        }
    }

    let is_running = get_domain_running(state, key);
    let edit_state = state.vm_windows[key].device_edits.disk.get(&idx).unwrap();
    let (apply, revert) = show_apply_revert(ui, edit_state.is_dirty(), is_running);

    if apply {
        let edit_state = state.vm_windows[key].device_edits.disk.get(&idx).unwrap();
        let new_dev_xml = if edit_state.sub_tab == DeviceSubTab::Xml {
            edit_state.device_xml_text.clone()
        } else {
            edit_state.fields.to_xml()
        };
        if let Some(new_xml) =
            xml_helpers::replace_nth_device_element(domain_xml, "disk", idx, &new_dev_xml)
        {
            send_define(state, key, new_xml);
        }
        let e = state.vm_windows.get_mut(key).unwrap().device_edits.disk.get_mut(&idx).unwrap();
        e.dirty = false;
        e.device_xml_dirty = false;
    }
    if revert {
        state.vm_windows.get_mut(key).unwrap().device_edits.disk.remove(&idx);
    }
}

pub fn show_nic_editor(
    ui: &mut Ui,
    state: &mut AppState,
    key: &str,
    guest: Option<&Guest>,
    idx: usize,
    domain_xml: &str,
) {
    let guest = match guest {
        Some(g) => g,
        None => return,
    };

    let caps = get_qemu_caps(state, key);
    let edit = state.vm_windows.get_mut(key).unwrap();
    if !edit.device_edits.nic.contains_key(&idx) {
        if let Some(ne) = NicEdit::from_guest(guest, idx) {
            edit.device_edits.nic.insert(idx, DeviceEditState::new(ne));
        } else {
            return;
        }
    }
    let edit_state = edit.device_edits.nic.get_mut(&idx).unwrap();

    show_sub_tabs(ui, &mut edit_state.sub_tab);

    match edit_state.sub_tab {
        DeviceSubTab::Details => {
            ui.heading("Network Interface");
            ui.separator();
            egui::Grid::new(format!("nic_edit_{idx}"))
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Type:");
                    let type_opts = vec!["network".into(), "bridge".into(), "direct".into()];
                    combo_box(
                        ui,
                        &format!("nic_type_{idx}"),
                        &mut edit_state.fields.interface_type,
                        &type_opts,
                        &mut edit_state.dirty,
                    );
                    ui.end_row();

                    let source_label = match edit_state.fields.interface_type.as_str() {
                        "bridge" => "Bridge:",
                        "direct" => "Device:",
                        _ => "Network:",
                    };
                    ui.label(source_label);
                    if ui
                        .text_edit_singleline(&mut edit_state.fields.source)
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Model:");
                    combo_box(
                        ui,
                        &format!("nic_model_{idx}"),
                        &mut edit_state.fields.model,
                        &caps.nic_models,
                        &mut edit_state.dirty,
                    );
                    ui.end_row();

                    ui.label("MAC Address:");
                    if ui
                        .text_edit_singleline(&mut edit_state.fields.mac_address)
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();
                });
        }
        DeviceSubTab::Xml => {
            if edit_state.device_xml_text.is_empty() {
                edit_state.device_xml_text =
                    xml_helpers::find_nth_device_element(domain_xml, "interface", idx)
                        .map(|s| s.content)
                        .unwrap_or_default();
            }
            show_device_xml_editor(
                ui,
                &mut edit_state.device_xml_text,
                &mut edit_state.device_xml_dirty,
            );
        }
    }

    let is_running = get_domain_running(state, key);
    let edit_state = state.vm_windows[key].device_edits.nic.get(&idx).unwrap();
    let (apply, revert) = show_apply_revert(ui, edit_state.is_dirty(), is_running);

    if apply {
        let edit_state = state.vm_windows[key].device_edits.nic.get(&idx).unwrap();
        let new_dev_xml = if edit_state.sub_tab == DeviceSubTab::Xml {
            edit_state.device_xml_text.clone()
        } else {
            edit_state.fields.to_xml()
        };
        if let Some(new_xml) =
            xml_helpers::replace_nth_device_element(domain_xml, "interface", idx, &new_dev_xml)
        {
            send_define(state, key, new_xml);
        }
        let e = state.vm_windows.get_mut(key).unwrap().device_edits.nic.get_mut(&idx).unwrap();
        e.dirty = false;
        e.device_xml_dirty = false;
    }
    if revert {
        state.vm_windows.get_mut(key).unwrap().device_edits.nic.remove(&idx);
    }
}

pub fn show_graphics_editor(
    ui: &mut Ui,
    state: &mut AppState,
    key: &str,
    guest: Option<&Guest>,
    idx: usize,
    domain_xml: &str,
) {
    let guest = match guest {
        Some(g) => g,
        None => return,
    };

    let edit = state.vm_windows.get_mut(key).unwrap();
    if !edit.device_edits.graphics.contains_key(&idx) {
        if let Some(ge) = GraphicsEdit::from_guest(guest, idx) {
            edit.device_edits.graphics.insert(idx, DeviceEditState::new(ge));
        } else {
            return;
        }
    }
    let edit_state = edit.device_edits.graphics.get_mut(&idx).unwrap();

    show_sub_tabs(ui, &mut edit_state.sub_tab);

    match edit_state.sub_tab {
        DeviceSubTab::Details => {
            ui.heading("Graphics");
            ui.separator();
            egui::Grid::new(format!("gfx_edit_{idx}"))
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Type:");
                    let type_opts = vec!["vnc".into(), "spice".into()];
                    combo_box(
                        ui,
                        &format!("gfx_type_{idx}"),
                        &mut edit_state.fields.graphics_type,
                        &type_opts,
                        &mut edit_state.dirty,
                    );
                    ui.end_row();

                    ui.label("Port:");
                    if ui
                        .add(egui::DragValue::new(&mut edit_state.fields.port).range(-1..=65535))
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Auto Port:");
                    if ui.checkbox(&mut edit_state.fields.autoport, "").changed() {
                        edit_state.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Listen:");
                    if ui
                        .text_edit_singleline(&mut edit_state.fields.listen)
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Password:");
                    if ui
                        .add(egui::TextEdit::singleline(&mut edit_state.fields.password).password(true))
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Keymap:");
                    if ui
                        .text_edit_singleline(&mut edit_state.fields.keymap)
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();
                });
        }
        DeviceSubTab::Xml => {
            if edit_state.device_xml_text.is_empty() {
                edit_state.device_xml_text =
                    xml_helpers::find_nth_device_element(domain_xml, "graphics", idx)
                        .map(|s| s.content)
                        .unwrap_or_default();
            }
            show_device_xml_editor(
                ui,
                &mut edit_state.device_xml_text,
                &mut edit_state.device_xml_dirty,
            );
        }
    }

    let is_running = get_domain_running(state, key);
    let edit_state = state.vm_windows[key].device_edits.graphics.get(&idx).unwrap();
    let (apply, revert) = show_apply_revert(ui, edit_state.is_dirty(), is_running);

    if apply {
        let edit_state = state.vm_windows[key].device_edits.graphics.get(&idx).unwrap();
        let new_dev_xml = if edit_state.sub_tab == DeviceSubTab::Xml {
            edit_state.device_xml_text.clone()
        } else {
            edit_state.fields.to_xml()
        };
        if let Some(new_xml) =
            xml_helpers::replace_nth_device_element(domain_xml, "graphics", idx, &new_dev_xml)
        {
            send_define(state, key, new_xml);
        }
        let e = state.vm_windows.get_mut(key).unwrap().device_edits.graphics.get_mut(&idx).unwrap();
        e.dirty = false;
        e.device_xml_dirty = false;
    }
    if revert {
        state.vm_windows.get_mut(key).unwrap().device_edits.graphics.remove(&idx);
    }
}

pub fn show_video_editor(
    ui: &mut Ui,
    state: &mut AppState,
    key: &str,
    guest: Option<&Guest>,
    idx: usize,
    domain_xml: &str,
) {
    let guest = match guest {
        Some(g) => g,
        None => return,
    };

    let caps = get_qemu_caps(state, key);
    let edit = state.vm_windows.get_mut(key).unwrap();
    if !edit.device_edits.video.contains_key(&idx) {
        if let Some(ve) = VideoEdit::from_guest(guest, idx) {
            edit.device_edits.video.insert(idx, DeviceEditState::new(ve));
        } else {
            return;
        }
    }
    let edit_state = edit.device_edits.video.get_mut(&idx).unwrap();

    show_sub_tabs(ui, &mut edit_state.sub_tab);

    match edit_state.sub_tab {
        DeviceSubTab::Details => {
            ui.heading("Video");
            ui.separator();
            egui::Grid::new(format!("video_edit_{idx}"))
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Model:");
                    combo_box(
                        ui,
                        &format!("vid_model_{idx}"),
                        &mut edit_state.fields.model_type,
                        &caps.video_models,
                        &mut edit_state.dirty,
                    );
                    ui.end_row();

                    ui.label("VRAM (KiB):");
                    if ui
                        .add(egui::DragValue::new(&mut edit_state.fields.vram_kib).range(256..=1048576))
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Heads:");
                    if ui
                        .add(egui::DragValue::new(&mut edit_state.fields.heads).range(1..=4))
                        .changed()
                    {
                        edit_state.dirty = true;
                    }
                    ui.end_row();

                    ui.label("3D Acceleration:");
                    if ui.checkbox(&mut edit_state.fields.accel3d, "").changed() {
                        edit_state.dirty = true;
                    }
                    ui.end_row();
                });
        }
        DeviceSubTab::Xml => {
            if edit_state.device_xml_text.is_empty() {
                edit_state.device_xml_text =
                    xml_helpers::find_nth_device_element(domain_xml, "video", idx)
                        .map(|s| s.content)
                        .unwrap_or_default();
            }
            show_device_xml_editor(
                ui,
                &mut edit_state.device_xml_text,
                &mut edit_state.device_xml_dirty,
            );
        }
    }

    let is_running = get_domain_running(state, key);
    let edit_state = state.vm_windows[key].device_edits.video.get(&idx).unwrap();
    let (apply, revert) = show_apply_revert(ui, edit_state.is_dirty(), is_running);

    if apply {
        let edit_state = state.vm_windows[key].device_edits.video.get(&idx).unwrap();
        let new_dev_xml = if edit_state.sub_tab == DeviceSubTab::Xml {
            edit_state.device_xml_text.clone()
        } else {
            edit_state.fields.to_xml()
        };
        if let Some(new_xml) =
            xml_helpers::replace_nth_device_element(domain_xml, "video", idx, &new_dev_xml)
        {
            send_define(state, key, new_xml);
        }
        let e = state.vm_windows.get_mut(key).unwrap().device_edits.video.get_mut(&idx).unwrap();
        e.dirty = false;
        e.device_xml_dirty = false;
    }
    if revert {
        state.vm_windows.get_mut(key).unwrap().device_edits.video.remove(&idx);
    }
}

pub fn show_sound_editor(
    ui: &mut Ui,
    state: &mut AppState,
    key: &str,
    guest: Option<&Guest>,
    idx: usize,
    domain_xml: &str,
) {
    let guest = match guest {
        Some(g) => g,
        None => return,
    };

    let caps = get_qemu_caps(state, key);
    let edit = state.vm_windows.get_mut(key).unwrap();
    if !edit.device_edits.sound.contains_key(&idx) {
        if let Some(se) = SoundEdit::from_guest(guest, idx) {
            edit.device_edits.sound.insert(idx, DeviceEditState::new(se));
        } else {
            return;
        }
    }
    let edit_state = edit.device_edits.sound.get_mut(&idx).unwrap();

    show_sub_tabs(ui, &mut edit_state.sub_tab);

    match edit_state.sub_tab {
        DeviceSubTab::Details => {
            ui.heading("Sound");
            ui.separator();
            egui::Grid::new(format!("snd_edit_{idx}"))
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Model:");
                    combo_box(
                        ui,
                        &format!("snd_model_{idx}"),
                        &mut edit_state.fields.model,
                        &caps.sound_models,
                        &mut edit_state.dirty,
                    );
                    ui.end_row();
                });
        }
        DeviceSubTab::Xml => {
            if edit_state.device_xml_text.is_empty() {
                edit_state.device_xml_text =
                    xml_helpers::find_nth_device_element(domain_xml, "sound", idx)
                        .map(|s| s.content)
                        .unwrap_or_default();
            }
            show_device_xml_editor(
                ui,
                &mut edit_state.device_xml_text,
                &mut edit_state.device_xml_dirty,
            );
        }
    }

    let is_running = get_domain_running(state, key);
    let edit_state = state.vm_windows[key].device_edits.sound.get(&idx).unwrap();
    let (apply, revert) = show_apply_revert(ui, edit_state.is_dirty(), is_running);

    if apply {
        let edit_state = state.vm_windows[key].device_edits.sound.get(&idx).unwrap();
        let new_dev_xml = if edit_state.sub_tab == DeviceSubTab::Xml {
            edit_state.device_xml_text.clone()
        } else {
            edit_state.fields.to_xml()
        };
        if let Some(new_xml) =
            xml_helpers::replace_nth_device_element(domain_xml, "sound", idx, &new_dev_xml)
        {
            send_define(state, key, new_xml);
        }
        let e = state.vm_windows.get_mut(key).unwrap().device_edits.sound.get_mut(&idx).unwrap();
        e.dirty = false;
        e.device_xml_dirty = false;
    }
    if revert {
        state.vm_windows.get_mut(key).unwrap().device_edits.sound.remove(&idx);
    }
}
