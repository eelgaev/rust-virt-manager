use egui::{Ui, ViewportBuilder, ViewportId};

use crate::backend::BackendCommand;
use crate::state::{AppState, BrowseTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwType {
    Disk,
    Nic,
    Graphics,
    Video,
    Sound,
    Input,
    Watchdog,
    Filesystem,
    Tpm,
    Rng,
    Serial,
}

impl HwType {
    pub const ALL: &[HwType] = &[
        HwType::Disk,
        HwType::Nic,
        HwType::Graphics,
        HwType::Video,
        HwType::Sound,
        HwType::Input,
        HwType::Watchdog,
        HwType::Filesystem,
        HwType::Tpm,
        HwType::Rng,
        HwType::Serial,
    ];

    fn label(&self) -> &'static str {
        match self {
            Self::Disk => "💿 Storage",
            Self::Nic => "🌐 Network",
            Self::Graphics => "🖵 Graphics",
            Self::Video => "🎬 Video",
            Self::Sound => "🔊 Sound",
            Self::Input => "🖱 Input",
            Self::Watchdog => "⏱ Watchdog",
            Self::Filesystem => "📁 Filesystem",
            Self::Tpm => "🔐 TPM",
            Self::Rng => "🎲 RNG",
            Self::Serial => "📟 Serial",
        }
    }
}

pub struct AddHardwareState {
    pub open: bool,
    pub vm_key: String,
    pub selected_type: HwType,
    pub disk_path: String,
    pub disk_size_gib: u64,
    pub disk_device: String,
    pub disk_bus: String,
    pub disk_create_new: bool,
    pub nic_type: String,
    pub nic_source: String,
    pub nic_model: String,
    pub gfx_type: String,
    pub gfx_port: String,
    pub video_model: String,
    pub sound_model: String,
    pub input_type: String,
    pub input_bus: String,
    pub watchdog_model: String,
    pub watchdog_action: String,
    pub fs_source: String,
    pub fs_target: String,
    pub fs_accessmode: String,
    pub tpm_model: String,
    pub tpm_version: String,
    pub rng_backend: String,
    pub serial_type: String,
}

impl Default for AddHardwareState {
    fn default() -> Self {
        Self {
            open: false,
            vm_key: String::new(),
            selected_type: HwType::Disk,
            disk_path: String::new(),
            disk_size_gib: 10,
            disk_device: "disk".into(),
            disk_bus: "virtio".into(),
            disk_create_new: true,
            nic_type: "network".into(),
            nic_source: "default".into(),
            nic_model: "virtio".into(),
            gfx_type: "vnc".into(),
            gfx_port: "-1".into(),
            video_model: "virtio".into(),
            sound_model: "ich9".into(),
            input_type: "tablet".into(),
            input_bus: "usb".into(),
            watchdog_model: "i6300esb".into(),
            watchdog_action: "reset".into(),
            fs_source: String::new(),
            fs_target: "shared".into(),
            fs_accessmode: "mapped".into(),
            tpm_model: "tpm-crb".into(),
            tpm_version: "2.0".into(),
            rng_backend: "/dev/urandom".into(),
            serial_type: "pty".into(),
        }
    }
}

pub fn show_add_hardware_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.add_hardware.open {
        return;
    }

    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("add_hardware"),
        ViewportBuilder::default()
            .with_title("Add New Virtual Hardware")
            .with_inner_size([550.0, 400.0]),
        |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                state.add_hardware = AddHardwareState::default();
            }
            egui::CentralPanel::default().show(ui.ctx(), |ui| {
                ui.columns(2, |cols| {
                    egui::ScrollArea::vertical()
                        .id_salt("hw_type_list")
                        .show(&mut cols[0], |ui| {
                            for &hw in HwType::ALL {
                                if ui
                                    .selectable_label(
                                        state.add_hardware.selected_type == hw,
                                        hw.label(),
                                    )
                                    .clicked()
                                {
                                    state.add_hardware.selected_type = hw;
                                }
                            }
                        });

                    egui::ScrollArea::vertical()
                        .id_salt("hw_config")
                        .show(&mut cols[1], |ui| {
                            match state.add_hardware.selected_type {
                                HwType::Disk => show_disk_config(ui, state),
                                HwType::Nic => show_nic_config(ui, state),
                                HwType::Graphics => show_gfx_config(ui, state),
                                HwType::Video => show_video_config(ui, state),
                                HwType::Sound => show_sound_config(ui, state),
                                HwType::Input => show_input_config(ui, state),
                                HwType::Watchdog => show_watchdog_config(ui, state),
                                HwType::Filesystem => show_fs_config(ui, state),
                                HwType::Tpm => show_tpm_config(ui, state),
                                HwType::Rng => show_rng_config(ui, state),
                                HwType::Serial => show_serial_config(ui, state),
                            }
                        });
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("✅ Add").clicked() {
                        apply_hardware(state);
                        state.add_hardware.open = false;
                    }
                    if ui.button("Cancel").clicked() {
                        state.add_hardware = AddHardwareState::default();
                    }
                });
            });
        },
    );
}

fn show_disk_config(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Storage");
    ui.add_space(4.0);

    egui::Grid::new("add_disk_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Create new:");
            ui.checkbox(&mut state.add_hardware.disk_create_new, "");
            ui.end_row();

            if state.add_hardware.disk_create_new {
                ui.label("Size (GiB):");
                let mut size = state.add_hardware.disk_size_gib as i64;
                ui.add(egui::DragValue::new(&mut size).range(1..=10000));
                state.add_hardware.disk_size_gib = size.max(1) as u64;
                ui.end_row();
            } else {
                ui.label("Path:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut state.add_hardware.disk_path);
                    if ui.button("📁").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Disk images", &["qcow2", "raw", "img", "vmdk"])
                            .pick_file()
                        {
                            state.add_hardware.disk_path =
                                path.to_string_lossy().to_string();
                        }
                    }
                    if ui.button("📦").on_hover_text("Browse storage pools").clicked() {
                        let uri = state.vm_windows.get(&state.add_hardware.vm_key)
                            .map(|vm| vm.uri.clone())
                            .unwrap_or_default();
                        state.volume_browser.open = true;
                        state.volume_browser.uri = uri;
                        state.volume_browser.target = BrowseTarget::AddHwDisk;
                        state.volume_browser.selected_path = None;
                        state.volume_browser.selected_pool = None;
                    }
                });
                ui.end_row();
            }

            if let Some(ref path) = state.volume_browser.selected_path.clone() {
                if !state.volume_browser.open
                    && state.volume_browser.target == BrowseTarget::AddHwDisk
                    && !path.is_empty()
                {
                    state.add_hardware.disk_path = path.clone();
                    state.add_hardware.disk_create_new = false;
                    state.volume_browser.selected_path = None;
                }
            }

            ui.label("Device type:");
            egui::ComboBox::from_id_salt("disk_dev_type")
                .selected_text(&state.add_hardware.disk_device)
                .show_ui(ui, |ui| {
                    for d in &["disk", "cdrom", "floppy"] {
                        ui.selectable_value(
                            &mut state.add_hardware.disk_device,
                            d.to_string(),
                            *d,
                        );
                    }
                });
            ui.end_row();

            ui.label("Bus:");
            egui::ComboBox::from_id_salt("disk_bus")
                .selected_text(&state.add_hardware.disk_bus)
                .show_ui(ui, |ui| {
                    for b in &["virtio", "scsi", "sata", "ide", "usb"] {
                        ui.selectable_value(
                            &mut state.add_hardware.disk_bus,
                            b.to_string(),
                            *b,
                        );
                    }
                });
            ui.end_row();
        });
}

fn show_nic_config(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Network");
    ui.add_space(4.0);

    egui::Grid::new("add_nic_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Network type:");
            egui::ComboBox::from_id_salt("nic_type")
                .selected_text(&state.add_hardware.nic_type)
                .show_ui(ui, |ui| {
                    for t in &["network", "bridge", "direct"] {
                        ui.selectable_value(
                            &mut state.add_hardware.nic_type,
                            t.to_string(),
                            *t,
                        );
                    }
                });
            ui.end_row();

            ui.label("Source:");
            ui.text_edit_singleline(&mut state.add_hardware.nic_source);
            ui.end_row();

            ui.label("Model:");
            egui::ComboBox::from_id_salt("nic_model")
                .selected_text(&state.add_hardware.nic_model)
                .show_ui(ui, |ui| {
                    for m in &["virtio", "e1000e", "e1000", "rtl8139"] {
                        ui.selectable_value(
                            &mut state.add_hardware.nic_model,
                            m.to_string(),
                            *m,
                        );
                    }
                });
            ui.end_row();
        });
}

fn show_gfx_config(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Graphics");
    ui.add_space(4.0);

    egui::Grid::new("add_gfx_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_salt("gfx_type")
                .selected_text(&state.add_hardware.gfx_type)
                .show_ui(ui, |ui| {
                    for t in &["vnc", "spice"] {
                        ui.selectable_value(
                            &mut state.add_hardware.gfx_type,
                            t.to_string(),
                            *t,
                        );
                    }
                });
            ui.end_row();

            ui.label("Port:");
            ui.text_edit_singleline(&mut state.add_hardware.gfx_port);
            ui.end_row();
        });
}

fn show_video_config(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Video");
    ui.add_space(4.0);

    egui::Grid::new("add_video_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Model:");
            egui::ComboBox::from_id_salt("video_model")
                .selected_text(&state.add_hardware.video_model)
                .show_ui(ui, |ui| {
                    for m in &["virtio", "qxl", "vga", "bochs", "cirrus", "none"] {
                        ui.selectable_value(
                            &mut state.add_hardware.video_model,
                            m.to_string(),
                            *m,
                        );
                    }
                });
            ui.end_row();
        });
}

fn show_sound_config(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Sound");
    ui.add_space(4.0);

    egui::Grid::new("add_sound_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Model:");
            egui::ComboBox::from_id_salt("sound_model")
                .selected_text(&state.add_hardware.sound_model)
                .show_ui(ui, |ui| {
                    for m in &["ich9", "ich6", "ac97", "es1370", "sb16"] {
                        ui.selectable_value(
                            &mut state.add_hardware.sound_model,
                            m.to_string(),
                            *m,
                        );
                    }
                });
            ui.end_row();
        });
}

fn show_input_config(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Input");
    ui.add_space(4.0);

    egui::Grid::new("add_input_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_salt("input_type")
                .selected_text(&state.add_hardware.input_type)
                .show_ui(ui, |ui| {
                    for t in &["tablet", "mouse", "keyboard"] {
                        ui.selectable_value(
                            &mut state.add_hardware.input_type,
                            t.to_string(),
                            *t,
                        );
                    }
                });
            ui.end_row();

            ui.label("Bus:");
            egui::ComboBox::from_id_salt("input_bus")
                .selected_text(&state.add_hardware.input_bus)
                .show_ui(ui, |ui| {
                    for b in &["usb", "virtio", "ps2"] {
                        ui.selectable_value(
                            &mut state.add_hardware.input_bus,
                            b.to_string(),
                            *b,
                        );
                    }
                });
            ui.end_row();
        });
}

fn show_watchdog_config(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Watchdog");
    ui.add_space(4.0);

    egui::Grid::new("add_watchdog_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Model:");
            egui::ComboBox::from_id_salt("wd_model")
                .selected_text(&state.add_hardware.watchdog_model)
                .show_ui(ui, |ui| {
                    for m in &["i6300esb", "diag288"] {
                        ui.selectable_value(
                            &mut state.add_hardware.watchdog_model,
                            m.to_string(),
                            *m,
                        );
                    }
                });
            ui.end_row();

            ui.label("Action:");
            egui::ComboBox::from_id_salt("wd_action")
                .selected_text(&state.add_hardware.watchdog_action)
                .show_ui(ui, |ui| {
                    for a in &["reset", "shutdown", "poweroff", "pause", "none", "inject-nmi"] {
                        ui.selectable_value(
                            &mut state.add_hardware.watchdog_action,
                            a.to_string(),
                            *a,
                        );
                    }
                });
            ui.end_row();
        });
}

fn show_fs_config(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Filesystem Passthrough");
    ui.add_space(4.0);

    egui::Grid::new("add_fs_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Source path:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut state.add_hardware.fs_source);
                if ui.button("📁").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        state.add_hardware.fs_source = path.to_string_lossy().to_string();
                    }
                }
            });
            ui.end_row();

            ui.label("Mount tag:");
            ui.text_edit_singleline(&mut state.add_hardware.fs_target);
            ui.end_row();

            ui.label("Access mode:");
            egui::ComboBox::from_id_salt("fs_access")
                .selected_text(&state.add_hardware.fs_accessmode)
                .show_ui(ui, |ui| {
                    for m in &["mapped", "passthrough", "squash"] {
                        ui.selectable_value(
                            &mut state.add_hardware.fs_accessmode,
                            m.to_string(),
                            *m,
                        );
                    }
                });
            ui.end_row();
        });
}

fn show_tpm_config(ui: &mut Ui, state: &mut AppState) {
    ui.heading("TPM");
    ui.add_space(4.0);

    egui::Grid::new("add_tpm_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Model:");
            egui::ComboBox::from_id_salt("tpm_model")
                .selected_text(&state.add_hardware.tpm_model)
                .show_ui(ui, |ui| {
                    for m in &["tpm-crb", "tpm-tis"] {
                        ui.selectable_value(
                            &mut state.add_hardware.tpm_model,
                            m.to_string(),
                            *m,
                        );
                    }
                });
            ui.end_row();

            ui.label("Version:");
            egui::ComboBox::from_id_salt("tpm_version")
                .selected_text(&state.add_hardware.tpm_version)
                .show_ui(ui, |ui| {
                    for v in &["2.0", "1.2"] {
                        ui.selectable_value(
                            &mut state.add_hardware.tpm_version,
                            v.to_string(),
                            *v,
                        );
                    }
                });
            ui.end_row();
        });
}

fn show_rng_config(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Random Number Generator");
    ui.add_space(4.0);

    egui::Grid::new("add_rng_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Backend:");
            egui::ComboBox::from_id_salt("rng_backend")
                .selected_text(&state.add_hardware.rng_backend)
                .show_ui(ui, |ui| {
                    for b in &["/dev/urandom", "/dev/random"] {
                        ui.selectable_value(
                            &mut state.add_hardware.rng_backend,
                            b.to_string(),
                            *b,
                        );
                    }
                });
            ui.end_row();
        });
}

fn show_serial_config(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Serial Port");
    ui.add_space(4.0);

    egui::Grid::new("add_serial_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_salt("serial_type")
                .selected_text(&state.add_hardware.serial_type)
                .show_ui(ui, |ui| {
                    for t in &["pty", "tcp", "file"] {
                        ui.selectable_value(
                            &mut state.add_hardware.serial_type,
                            t.to_string(),
                            *t,
                        );
                    }
                });
            ui.end_row();
        });
}

fn apply_hardware(state: &mut AppState) {
    let vm_key = state.add_hardware.vm_key.clone();
    let vm = match state.vm_windows.get(&vm_key) {
        Some(vm) => vm,
        None => return,
    };
    let uri = vm.uri.clone();
    let domain_name = vm.domain_name.clone();

    let xml = state
        .connections
        .iter()
        .find(|c| c.uri == uri)
        .and_then(|c| c.domains.get(&domain_name))
        .map(|d| d.xml.clone())
        .unwrap_or_default();

    let device_xml = match state.add_hardware.selected_type {
        HwType::Disk => {
            let hw = &state.add_hardware;
            let target_dev = match hw.disk_bus.as_str() {
                "virtio" => "vdb",
                "scsi" => "sdb",
                "sata" => "sdb",
                _ => "hdb",
            };
            if hw.disk_create_new {
                format!(
                    "<disk type='file' device='{}'>\
                       <driver name='qemu' type='qcow2'/>\
                       <source file='/var/lib/libvirt/images/{domain_name}-added.qcow2'/>\
                       <target dev='{target_dev}' bus='{}'/>\
                     </disk>",
                    hw.disk_device, hw.disk_bus,
                )
            } else {
                format!(
                    "<disk type='file' device='{}'>\
                       <driver name='qemu' type='qcow2'/>\
                       <source file='{}'/>\
                       <target dev='{target_dev}' bus='{}'/>\
                     </disk>",
                    hw.disk_device, hw.disk_path, hw.disk_bus,
                )
            }
        }
        HwType::Nic => {
            let hw = &state.add_hardware;
            let source_attr = match hw.nic_type.as_str() {
                "bridge" => format!("bridge='{}'", hw.nic_source),
                "direct" => format!("dev='{}'", hw.nic_source),
                _ => format!("network='{}'", hw.nic_source),
            };
            format!(
                "<interface type='{}'>\
                   <source {source_attr}/>\
                   <model type='{}'/>\
                 </interface>",
                hw.nic_type, hw.nic_model,
            )
        }
        HwType::Graphics => {
            let hw = &state.add_hardware;
            format!(
                "<graphics type='{}' port='{}' autoport='yes'/>",
                hw.gfx_type, hw.gfx_port,
            )
        }
        HwType::Video => {
            format!(
                "<video><model type='{}' heads='1'/></video>",
                state.add_hardware.video_model,
            )
        }
        HwType::Sound => {
            format!("<sound model='{}'/>", state.add_hardware.sound_model)
        }
        HwType::Input => {
            format!(
                "<input type='{}' bus='{}'/>",
                state.add_hardware.input_type, state.add_hardware.input_bus,
            )
        }
        HwType::Watchdog => {
            format!(
                "<watchdog model='{}' action='{}'/>",
                state.add_hardware.watchdog_model, state.add_hardware.watchdog_action,
            )
        }
        HwType::Filesystem => {
            let hw = &state.add_hardware;
            format!(
                "<filesystem type='mount' accessmode='{}'>\
                   <source dir='{}'/>\
                   <target dir='{}'/>\
                 </filesystem>",
                hw.fs_accessmode, hw.fs_source, hw.fs_target,
            )
        }
        HwType::Tpm => {
            let hw = &state.add_hardware;
            format!(
                "<tpm model='{}'>\
                   <backend type='emulator' version='{}'/>\
                 </tpm>",
                hw.tpm_model, hw.tpm_version,
            )
        }
        HwType::Rng => {
            format!(
                "<rng model='virtio'>\
                   <backend model='random'>{}</backend>\
                 </rng>",
                state.add_hardware.rng_backend,
            )
        }
        HwType::Serial => {
            format!("<serial type='{}'><target port='0'/></serial>", state.add_hardware.serial_type)
        }
    };

    let new_xml = inject_device_into_xml(&xml, &device_xml);
    state
        .backend
        .send_to(&uri, BackendCommand::DefineXml(uri.clone(), new_xml));

    state.add_hardware = AddHardwareState::default();
}

fn inject_device_into_xml(domain_xml: &str, device_xml: &str) -> String {
    if let Some(pos) = domain_xml.rfind("</devices>") {
        let mut result = domain_xml[..pos].to_string();
        result.push_str(device_xml);
        result.push_str(&domain_xml[pos..]);
        result
    } else {
        domain_xml.to_string()
    }
}
