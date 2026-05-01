use egui::{Ui, ViewportBuilder, ViewportId};

use crate::backend::BackendCommand;
use crate::state::{AppState, BrowseTarget};

const INSTALL_METHODS: [&str; 3] = ["Local ISO", "Import existing disk", "Manual install"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsPreset {
    GenericLinux,
    Fedora,
    Ubuntu,
    Debian,
    CentOS,
    Arch,
    Windows10,
    Windows11,
    WindowsServer,
    FreeBSD,
    Other,
}

impl OsPreset {
    const ALL: &[OsPreset] = &[
        Self::GenericLinux,
        Self::Fedora,
        Self::Ubuntu,
        Self::Debian,
        Self::CentOS,
        Self::Arch,
        Self::Windows10,
        Self::Windows11,
        Self::WindowsServer,
        Self::FreeBSD,
        Self::Other,
    ];

    fn label(&self) -> &'static str {
        match self {
            Self::GenericLinux => "Generic Linux",
            Self::Fedora => "Fedora",
            Self::Ubuntu => "Ubuntu",
            Self::Debian => "Debian",
            Self::CentOS => "CentOS / RHEL",
            Self::Arch => "Arch Linux",
            Self::Windows10 => "Windows 10",
            Self::Windows11 => "Windows 11",
            Self::WindowsServer => "Windows Server",
            Self::FreeBSD => "FreeBSD",
            Self::Other => "Other",
        }
    }

    fn recommended_vcpus(&self) -> u32 {
        match self {
            Self::Windows11 | Self::WindowsServer => 4,
            Self::Windows10 | Self::Fedora | Self::Ubuntu => 2,
            _ => 2,
        }
    }

    fn recommended_memory_mib(&self) -> u64 {
        match self {
            Self::Windows11 | Self::WindowsServer => 4096,
            Self::Windows10 => 4096,
            Self::Fedora | Self::Ubuntu => 2048,
            _ => 2048,
        }
    }

    fn recommended_disk_gib(&self) -> u64 {
        match self {
            Self::Windows11 | Self::WindowsServer => 64,
            Self::Windows10 => 40,
            Self::Fedora | Self::Ubuntu | Self::Debian | Self::CentOS => 20,
            _ => 20,
        }
    }

    fn needs_uefi(&self) -> bool {
        matches!(self, Self::Windows11)
    }

    fn use_virtio(&self) -> bool {
        !matches!(self, Self::FreeBSD | Self::Other)
    }
}

pub fn show_create_vm_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.create_vm.open {
        return;
    }

    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("create_vm"),
        ViewportBuilder::default()
            .with_title("Create a New Virtual Machine")
            .with_inner_size([550.0, 450.0]),
        |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                state.create_vm = Default::default();
            }
            egui::CentralPanel::default().show(ui.ctx(), |ui| {
                let page = state.create_vm.page;
                ui.horizontal(|ui| {
                    for (i, label) in ["Name", "Install", "CPU/Memory", "Storage", "Summary"]
                        .iter()
                        .enumerate()
                    {
                        if i == page {
                            ui.strong(*label);
                        } else {
                            ui.weak(*label);
                        }
                        if i < 4 {
                            ui.weak(">");
                        }
                    }
                });
                ui.separator();

                match page {
                    0 => show_page_name(ui, state),
                    1 => show_page_install(ui, state),
                    2 => show_page_cpu_memory(ui, state),
                    3 => show_page_storage(ui, state),
                    4 => show_page_summary(ui, state),
                    _ => {}
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if page > 0 && ui.button("⬅ Back").clicked() {
                        state.create_vm.page -= 1;
                    }

                    if page < 4 && ui.button("Next ➡").clicked() {
                        state.create_vm.page += 1;
                    }

                    if page == 4 {
                        if ui.button("✅ Finish").clicked() {
                            create_vm(state);
                            state.create_vm.open = false;
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        state.create_vm = Default::default();
                    }
                });
            });
        },
    );
}

fn show_page_name(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Name, OS & Install Method");
    ui.add_space(8.0);

    egui::Grid::new("name_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Connection:");
            let uris: Vec<String> = state.connections.iter().map(|c| c.uri.clone()).collect();
            egui::ComboBox::from_id_salt("conn_combo")
                .selected_text(&state.create_vm.uri)
                .show_ui(ui, |ui| {
                    for u in &uris {
                        ui.selectable_value(&mut state.create_vm.uri, u.clone(), u);
                    }
                });
            ui.end_row();

            ui.label("Name:");
            ui.text_edit_singleline(&mut state.create_vm.name);
            ui.end_row();

            ui.label("OS Type:");
            let current_label = state.create_vm.os_preset.label();
            let mut changed = false;
            egui::ComboBox::from_id_salt("os_preset")
                .selected_text(current_label)
                .show_ui(ui, |ui| {
                    for &os in OsPreset::ALL {
                        if ui
                            .selectable_value(&mut state.create_vm.os_preset, os, os.label())
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            if changed {
                let preset = state.create_vm.os_preset;
                state.create_vm.vcpus = preset.recommended_vcpus();
                state.create_vm.memory_mib = preset.recommended_memory_mib();
                state.create_vm.disk_size_gib = preset.recommended_disk_gib();
            }

            ui.label("Install Method:");
            egui::ComboBox::from_id_salt("install_method")
                .selected_text(INSTALL_METHODS[state.create_vm.install_method])
                .show_ui(ui, |ui| {
                    for (i, m) in INSTALL_METHODS.iter().enumerate() {
                        ui.selectable_value(&mut state.create_vm.install_method, i, *m);
                    }
                });
            ui.end_row();
        });
}

fn show_page_install(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Install Source");
    ui.add_space(8.0);

    match state.create_vm.install_method {
        0 => {
            ui.label("ISO Image Path:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut state.create_vm.iso_path);
                if ui.button("📁 Browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("ISO images", &["iso", "img"])
                        .pick_file()
                    {
                        state.create_vm.iso_path = path.to_string_lossy().to_string();
                    }
                }
                if ui.button("📦 Pools").on_hover_text("Browse storage pools").clicked() {
                    state.volume_browser.open = true;
                    state.volume_browser.uri = state.create_vm.uri.clone();
                    state.volume_browser.target = BrowseTarget::IsoSource;
                    state.volume_browser.selected_path = None;
                    state.volume_browser.selected_pool = None;
                }
            });

            if let Some(ref path) = state.volume_browser.selected_path.clone() {
                if !state.volume_browser.open
                    && state.volume_browser.target == BrowseTarget::IsoSource
                    && !path.is_empty()
                {
                    state.create_vm.iso_path = path.clone();
                    state.volume_browser.selected_path = None;
                }
            }
        }
        1 => {
            ui.label("Existing Disk Path:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut state.create_vm.import_path);
                if ui.button("📁 Browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Disk images", &["qcow2", "raw", "img", "vmdk"])
                        .pick_file()
                    {
                        state.create_vm.import_path = path.to_string_lossy().to_string();
                    }
                }
                if ui.button("📦 Pools").on_hover_text("Browse storage pools").clicked() {
                    state.volume_browser.open = true;
                    state.volume_browser.uri = state.create_vm.uri.clone();
                    state.volume_browser.target = BrowseTarget::DiskSource;
                    state.volume_browser.selected_path = None;
                    state.volume_browser.selected_pool = None;
                }
            });

            if let Some(ref path) = state.volume_browser.selected_path.clone() {
                if !state.volume_browser.open
                    && state.volume_browser.target == BrowseTarget::DiskSource
                    && !path.is_empty()
                {
                    state.create_vm.import_path = path.clone();
                    state.volume_browser.selected_path = None;
                }
            }
        }
        2 => {
            ui.label("Manual install - no installation media will be attached.");
        }
        _ => {}
    }
}

fn show_page_cpu_memory(ui: &mut Ui, state: &mut AppState) {
    ui.heading("CPU & Memory");
    ui.add_space(8.0);

    egui::Grid::new("cpu_mem_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("vCPUs:");
            let mut vcpus = state.create_vm.vcpus as i32;
            ui.add(egui::DragValue::new(&mut vcpus).range(1..=128));
            state.create_vm.vcpus = vcpus.max(1) as u32;
            ui.end_row();

            ui.label("Memory (MiB):");
            let mut mem = state.create_vm.memory_mib as i64;
            ui.add(egui::DragValue::new(&mut mem).range(256..=1048576).speed(256.0));
            state.create_vm.memory_mib = mem.max(256) as u64;
            ui.end_row();
        });
}

fn show_page_storage(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Storage");
    ui.add_space(8.0);

    if state.create_vm.install_method == 1 {
        ui.label("Using imported disk, no additional storage needed.");
        state.create_vm.create_disk = false;
        return;
    }

    ui.checkbox(&mut state.create_vm.create_disk, "Create a disk image");

    if state.create_vm.create_disk {
        egui::Grid::new("storage_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label("Disk Size (GiB):");
                let mut size = state.create_vm.disk_size_gib as i64;
                ui.add(egui::DragValue::new(&mut size).range(1..=10000).speed(1.0));
                state.create_vm.disk_size_gib = size.max(1) as u64;
                ui.end_row();

                ui.label("Storage Pool:");
                let pools: Vec<String> = state.conn_details
                    .get(&state.create_vm.uri)
                    .map(|c| c.pools.iter().filter(|p| p.active).map(|p| p.name.clone()).collect())
                    .unwrap_or_default();
                if pools.is_empty() {
                    ui.label(&state.create_vm.target_pool);
                } else {
                    egui::ComboBox::from_id_salt("target_pool")
                        .selected_text(&state.create_vm.target_pool)
                        .show_ui(ui, |ui| {
                            for p in &pools {
                                ui.selectable_value(&mut state.create_vm.target_pool, p.clone(), p);
                            }
                        });
                }
                ui.end_row();
            });
    }
}

fn show_page_summary(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Summary");
    ui.add_space(8.0);

    let vm = &state.create_vm;
    egui::Grid::new("summary_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label("Name:");
            ui.label(&vm.name);
            ui.end_row();

            ui.label("Connection:");
            ui.label(&vm.uri);
            ui.end_row();

            ui.label("OS Type:");
            ui.label(vm.os_preset.label());
            ui.end_row();

            ui.label("Install:");
            ui.label(INSTALL_METHODS[vm.install_method]);
            ui.end_row();

            ui.label("vCPUs:");
            ui.label(format!("{}", vm.vcpus));
            ui.end_row();

            ui.label("Memory:");
            ui.label(format!("{} MiB", vm.memory_mib));
            ui.end_row();

            ui.label("Disk:");
            if vm.create_disk {
                ui.label(format!("{} GiB (pool: {})", vm.disk_size_gib, vm.target_pool));
            } else if vm.install_method == 1 {
                ui.label(&vm.import_path);
            } else {
                ui.label("None");
            }
            ui.end_row();

            if vm.os_preset.needs_uefi() {
                ui.label("Firmware:");
                ui.label("UEFI (required by OS)");
                ui.end_row();
            }
        });

    ui.add_space(8.0);
    ui.checkbox(
        &mut state.create_vm.customize_before_install,
        "Customize configuration before install",
    );
}

fn create_vm(state: &mut AppState) {
    let vm = &state.create_vm;
    let name = &vm.name;
    let vcpus = vm.vcpus;
    let memory_kib = vm.memory_mib * 1024;
    let preset = vm.os_preset;

    let disk_bus = if preset.use_virtio() { "virtio" } else { "sata" };
    let disk_target = if preset.use_virtio() { "vda" } else { "sda" };
    let nic_model = if preset.use_virtio() { "virtio" } else { "e1000e" };

    let disk_pool_path = state.conn_details.get(&vm.uri)
        .and_then(|c| c.pools.iter().find(|p| p.name == vm.target_pool))
        .and_then(|p| {
            let xml = &p.xml;
            xml.find("<path>")
                .and_then(|s| xml[s + 6..].find("</path>").map(|e| xml[s + 6..s + 6 + e].to_string()))
        })
        .unwrap_or_else(|| "/var/lib/libvirt/images".into());

    let disk_xml = if vm.create_disk {
        format!(
            "<disk type='file' device='disk'>\
               <driver name='qemu' type='qcow2'/>\
               <source file='{disk_pool_path}/{name}.qcow2'/>\
               <target dev='{disk_target}' bus='{disk_bus}'/>\
             </disk>"
        )
    } else if vm.install_method == 1 {
        format!(
            "<disk type='file' device='disk'>\
               <driver name='qemu' type='qcow2'/>\
               <source file='{}'/>\
               <target dev='{disk_target}' bus='{disk_bus}'/>\
             </disk>",
            vm.import_path
        )
    } else {
        String::new()
    };

    let cdrom_xml = if vm.install_method == 0 && !vm.iso_path.is_empty() {
        format!(
            "<disk type='file' device='cdrom'>\
               <driver name='qemu' type='raw'/>\
               <source file='{}'/>\
               <target dev='sdb' bus='sata'/>\
               <readonly/>\
             </disk>",
            vm.iso_path
        )
    } else {
        String::new()
    };

    let firmware_xml = if preset.needs_uefi() {
        "<loader readonly='yes' type='pflash'>/usr/share/edk2/x64/OVMF_CODE.fd</loader>\
         <nvram template='/usr/share/edk2/x64/OVMF_VARS.fd'/>"
    } else {
        ""
    };

    let tpm_xml = if matches!(preset, OsPreset::Windows11) {
        "<tpm model='tpm-crb'><backend type='emulator' version='2.0'/></tpm>"
    } else {
        ""
    };

    let domain_xml = format!(
        "<domain type='kvm'>\
           <name>{name}</name>\
           <memory unit='KiB'>{memory_kib}</memory>\
           <vcpu>{vcpus}</vcpu>\
           <os>\
             <type arch='x86_64' machine='pc-q35-9.0'>hvm</type>\
             {firmware_xml}\
             <boot dev='cdrom'/>\
             <boot dev='hd'/>\
           </os>\
           <devices>\
             {disk_xml}\
             {cdrom_xml}\
             <interface type='network'>\
               <source network='default'/>\
               <model type='{nic_model}'/>\
             </interface>\
             <graphics type='vnc' port='-1' autoport='yes'/>\
             <video>\
               <model type='virtio' heads='1'/>\
             </video>\
             <input type='tablet' bus='usb'/>\
             <console type='pty'/>\
             {tpm_xml}\
           </devices>\
         </domain>"
    );

    let uri = vm.uri.clone();
    state
        .backend
        .send_to(&uri, BackendCommand::DefineXml(uri.clone(), domain_xml));
}
