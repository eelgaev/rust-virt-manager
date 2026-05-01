use egui::{RichText, Ui, ViewportBuilder, ViewportId};

use crate::backend::BackendCommand;
use crate::state::AppState;

pub fn show_volume_browser_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.volume_browser.open {
        return;
    }

    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("volume_browser"),
        ViewportBuilder::default()
            .with_title("Browse Storage Volumes")
            .with_inner_size([650.0, 400.0]),
        |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                state.volume_browser.open = false;
            }
            egui::CentralPanel::default().show(ui.ctx(), |ui| {
                show_browser_content(ui, state);
            });
        },
    );
}

fn show_browser_content(ui: &mut Ui, state: &mut AppState) {
    let uri = state.volume_browser.uri.clone();

    let pools = state.conn_details.get(&uri)
        .map(|c| c.pools.clone())
        .unwrap_or_default();

    if pools.is_empty() {
        state.backend.send_to(&uri, BackendCommand::ListStoragePools(uri.clone()));
        ui.label("Loading storage pools...");
        return;
    }

    ui.columns(2, |cols| {
        egui::ScrollArea::vertical()
            .id_salt("browser_pools")
            .show(&mut cols[0], |ui| {
                ui.label(RichText::new("Storage Pools").strong());
                ui.separator();

                for pool in &pools {
                    let label = if pool.active {
                        format!("▶ {}", pool.name)
                    } else {
                        format!("⏹ {} (inactive)", pool.name)
                    };
                    let is_selected = state.volume_browser.selected_pool.as_deref() == Some(&pool.name);
                    if ui.selectable_label(is_selected, &label).clicked() {
                        state.volume_browser.selected_pool = Some(pool.name.clone());
                        state.backend.send_to(
                            &uri,
                            BackendCommand::ListVolumes(uri.clone(), pool.name.clone()),
                        );
                    }
                }
            });

        let sel_pool = state.volume_browser.selected_pool.clone();
        egui::ScrollArea::vertical()
            .id_salt("browser_vols")
            .show(&mut cols[1], |ui| {
                if let Some(pool_name) = &sel_pool {
                    ui.label(RichText::new(format!("Volumes in '{pool_name}'")).strong());
                    ui.separator();

                    let vols = state.conn_details.get(&uri)
                        .and_then(|c| c.volumes.get(pool_name))
                        .cloned()
                        .unwrap_or_default();

                    if vols.is_empty() {
                        ui.label("No volumes.");
                    } else {
                        for vol in &vols {
                            let label = format!(
                                "{} ({}, {:.1} GiB)",
                                vol.name,
                                vol.vol_type,
                                vol.capacity as f64 / 1_073_741_824.0,
                            );
                            let is_selected = state.volume_browser.selected_path.as_deref() == Some(&vol.path);
                            let resp = ui.selectable_label(is_selected, &label);
                            if resp.clicked() {
                                state.volume_browser.selected_path = Some(vol.path.clone());
                            }
                            if resp.double_clicked() {
                                state.volume_browser.selected_path = Some(vol.path.clone());
                                state.volume_browser.open = false;
                            }
                        }
                    }
                } else {
                    ui.label("Select a pool to browse volumes.");
                }
            });
    });

    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Selected:");
        if let Some(path) = &state.volume_browser.selected_path {
            ui.label(RichText::new(path).monospace());
        } else {
            ui.label("(none)");
        }
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let has_selection = state.volume_browser.selected_path.is_some();
        if ui.add_enabled(has_selection, egui::Button::new("✅ Select")).clicked() {
            state.volume_browser.open = false;
        }
        if ui.button("❌ Cancel").clicked() {
            state.volume_browser.selected_path = None;
            state.volume_browser.open = false;
        }
    });
}
