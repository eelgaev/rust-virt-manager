use egui::{Ui, ViewportBuilder, ViewportId};

use crate::backend::BackendCommand;
use crate::domain::Guest;
use crate::state::AppState;

pub fn show_clone_vm_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.clone_vm.open {
        return;
    }

    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("clone_vm"),
        ViewportBuilder::default()
            .with_title("Clone Virtual Machine")
            .with_inner_size([500.0, 400.0]),
        |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                state.clone_vm = Default::default();
            }
            egui::CentralPanel::default().show(ui.ctx(), |ui| {
                show_clone_form(ui, state);

                ui.separator();
                ui.horizontal(|ui| {
                    let name_empty = state.clone_vm.clone_name.is_empty();
                    if ui
                        .add_enabled(!name_empty, egui::Button::new("✅ Clone"))
                        .clicked()
                    {
                        do_clone(state);
                        state.clone_vm.open = false;
                    }
                    if ui.button("Cancel").clicked() {
                        state.clone_vm = Default::default();
                    }
                });
            });
        },
    );
}

fn show_clone_form(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Clone Configuration");
    ui.add_space(8.0);

    let xml = state
        .connections
        .iter()
        .find(|c| c.uri == state.clone_vm.uri)
        .and_then(|c| c.domains.get(&state.clone_vm.source_name))
        .map(|d| d.xml.clone())
        .unwrap_or_default();

    let guest = Guest::from_xml(&xml).ok();

    egui::Grid::new("clone_form_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Original VM:");
            ui.label(&state.clone_vm.source_name);
            ui.end_row();

            ui.label("New Name:");
            ui.text_edit_singleline(&mut state.clone_vm.clone_name);
            ui.end_row();
        });

    if let Some(guest) = &guest {
        if let Some(devices) = &guest.devices {
            if !devices.disks.is_empty() {
                ui.add_space(8.0);
                ui.heading("Disks");
                ui.separator();

                for (i, disk) in devices.disks.iter().enumerate() {
                    let path = disk.source_path().unwrap_or("N/A");
                    ui.horizontal(|ui| {
                        ui.label(format!("{}.", i + 1));
                        ui.label(path);
                    });

                    if i < state.clone_vm.disk_strategies.len() {
                        ui.horizontal(|ui| {
                            ui.label("  Strategy:");
                            egui::ComboBox::from_id_salt(format!("clone_disk_{i}"))
                                .selected_text(state.clone_vm.disk_strategies[i].label())
                                .show_ui(ui, |ui| {
                                    for s in &[
                                        CloneDiskStrategy::Clone,
                                        CloneDiskStrategy::Share,
                                    ] {
                                        ui.selectable_value(
                                            &mut state.clone_vm.disk_strategies[i],
                                            *s,
                                            s.label(),
                                        );
                                    }
                                });
                        });
                    }
                }
            }

            if !devices.interfaces.is_empty() {
                ui.add_space(8.0);
                ui.heading("Network Interfaces");
                ui.separator();
                ui.label("New MAC addresses will be generated automatically.");
            }
        }
    }
}

fn do_clone(state: &mut AppState) {
    let xml = state
        .connections
        .iter()
        .find(|c| c.uri == state.clone_vm.uri)
        .and_then(|c| c.domains.get(&state.clone_vm.source_name))
        .map(|d| d.xml.clone())
        .unwrap_or_default();

    if xml.is_empty() {
        return;
    }

    let new_name = &state.clone_vm.clone_name;
    let new_uuid = uuid::Uuid::new_v4().to_string();

    let mut new_xml = xml.clone();

    if let Some(start) = new_xml.find("<name>") {
        if let Some(end) = new_xml.find("</name>") {
            new_xml = format!(
                "{}<name>{new_name}</name>{}",
                &new_xml[..start],
                &new_xml[end + "</name>".len()..],
            );
        }
    }

    if let Some(start) = new_xml.find("<uuid>") {
        if let Some(end) = new_xml.find("</uuid>") {
            new_xml = format!(
                "{}<uuid>{new_uuid}</uuid>{}",
                &new_xml[..start],
                &new_xml[end + "</uuid>".len()..],
            );
        }
    }

    let re_mac = |xml: &str| -> String {
        let mut result = xml.to_string();
        while let Some(pos) = result.find("<mac address='") {
            let after = pos + "<mac address='".len();
            if let Some(end) = result[after..].find("'") {
                let new_mac = generate_mac();
                result = format!(
                    "{}<mac address='{new_mac}'{}",
                    &result[..pos],
                    &result[after + end..],
                );
            } else {
                break;
            }
        }
        result
    };

    new_xml = re_mac(&new_xml);

    let uri = state.clone_vm.uri.clone();
    state
        .backend
        .send_to(&uri, BackendCommand::DefineXml(uri.clone(), new_xml));
}

fn generate_mac() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    format!(
        "52:54:00:{:02x}:{:02x}:{:02x}",
        rng.random::<u8>(),
        rng.random::<u8>(),
        rng.random::<u8>(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneDiskStrategy {
    Clone,
    Share,
}

impl CloneDiskStrategy {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Clone => "Clone disk",
            Self::Share => "Share disk (read-only)",
        }
    }
}
