use egui::{RichText, Ui, ViewportBuilder, ViewportId};

use crate::backend::BackendCommand;
use crate::state::AppState;

pub fn show_snapshots_tab(ui: &mut Ui, state: &mut AppState, key: &str) {
    let (uri, domain_name) = {
        let vm = &state.vm_windows[key];
        (vm.uri.clone(), vm.domain_name.clone())
    };

    ui.horizontal(|ui| {
        if ui.button("🔄 Refresh").clicked() {
            state.backend.send_to(
                &uri,
                BackendCommand::ListSnapshots(uri.clone(), domain_name.clone()),
            );
        }
        if ui.button("➕ New Snapshot").clicked() {
            let vm = state.vm_windows.get_mut(key).unwrap();
            vm.show_create_snapshot = true;
            vm.create_snapshot_name = format!("snapshot{}", vm.snapshots.len() + 1);
            vm.create_snapshot_desc.clear();
        }

        let has_selection = state.vm_windows[key].selected_snapshot.is_some();

        if ui
            .add_enabled(has_selection, egui::Button::new("⏪ Revert"))
            .clicked()
        {
            if let Some(snap_name) = state.vm_windows[key].selected_snapshot.clone() {
                state.backend.send_to(
                    &uri,
                    BackendCommand::RevertSnapshot(uri.clone(), domain_name.clone(), snap_name),
                );
            }
        }

        if ui
            .add_enabled(has_selection, egui::Button::new("🗑 Delete"))
            .clicked()
        {
            if let Some(snap_name) = state.vm_windows[key].selected_snapshot.clone() {
                state.backend.send_to(
                    &uri,
                    BackendCommand::DeleteSnapshot(uri.clone(), domain_name.clone(), snap_name),
                );
                state.vm_windows.get_mut(key).unwrap().selected_snapshot = None;
            }
        }
    });

    ui.separator();

    let snapshots = state.vm_windows[key].snapshots.clone();

    if snapshots.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("No snapshots. Click 'Refresh' to load or 'New Snapshot' to create one.");
        });
    } else {
        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_salt("snap_list")
                .show(&mut cols[0], |ui| {
                    for snap in &snapshots {
                        let is_selected = state.vm_windows[key]
                            .selected_snapshot
                            .as_deref()
                            == Some(&snap.name);
                        let label = if snap.is_current {
                            format!("● {}", snap.name)
                        } else {
                            snap.name.clone()
                        };
                        if ui.selectable_label(is_selected, &label).clicked() {
                            state.vm_windows.get_mut(key).unwrap().selected_snapshot =
                                Some(snap.name.clone());
                        }
                    }
                });

            egui::ScrollArea::vertical()
                .id_salt("snap_detail")
                .show(&mut cols[1], |ui| {
                    let selected = state.vm_windows[key].selected_snapshot.clone();
                    if let Some(sel_name) = &selected {
                        if let Some(snap) = snapshots.iter().find(|s| &s.name == sel_name) {
                            ui.heading(&snap.name);
                            ui.separator();

                            egui::Grid::new("snap_info")
                                .num_columns(2)
                                .spacing([12.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label("State:");
                                    ui.label(&snap.state);
                                    ui.end_row();

                                    ui.label("Created:");
                                    let dt = chrono_from_ts(snap.creation_time);
                                    ui.label(&dt);
                                    ui.end_row();

                                    if snap.is_current {
                                        ui.label("Current:");
                                        ui.label(RichText::new("Yes").strong());
                                        ui.end_row();
                                    }
                                });

                            if !snap.description.is_empty() {
                                ui.add_space(8.0);
                                ui.label(RichText::new("Description").strong());
                                ui.label(&snap.description);
                            }
                        }
                    } else {
                        ui.label("Select a snapshot to view details.");
                    }
                });
        });
    }
}

pub fn show_create_snapshot_window(
    ctx: &egui::Context,
    state: &mut AppState,
    key: &str,
    uri: &str,
    domain_name: &str,
) {
    let show = state.vm_windows.get(key).is_some_and(|vm| vm.show_create_snapshot);
    if !show {
        return;
    }

    ctx.show_viewport_immediate(
        ViewportId::from_hash_of(format!("create_snap_{key}")),
        ViewportBuilder::default()
            .with_title("Create Snapshot")
            .with_inner_size([400.0, 250.0]),
        |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                state.vm_windows.get_mut(key).unwrap().show_create_snapshot = false;
            }
            egui::CentralPanel::default().show(ui.ctx(), |ui| {
                let vm = state.vm_windows.get_mut(key).unwrap();

                egui::Grid::new("create_snap_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut vm.create_snapshot_name);
                        ui.end_row();

                        ui.label("Description:");
                        ui.text_edit_multiline(&mut vm.create_snapshot_desc);
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        state.vm_windows.get_mut(key).unwrap().show_create_snapshot = false;
                    }
                    if ui.button("✅ Create").clicked() {
                        let vm = &state.vm_windows[key];
                        let xml = format!(
                            "<domainsnapshot><name>{}</name><description>{}</description></domainsnapshot>",
                            vm.create_snapshot_name, vm.create_snapshot_desc
                        );
                        state.backend.send_to(
                            uri,
                            BackendCommand::CreateSnapshot(
                                uri.to_string(),
                                domain_name.to_string(),
                                xml,
                            ),
                        );
                        state.vm_windows.get_mut(key).unwrap().show_create_snapshot = false;
                    }
                });
            });
        },
    );
}

fn chrono_from_ts(ts: i64) -> String {
    if ts == 0 {
        return "Unknown".to_string();
    }
    format!("{ts}")
}
