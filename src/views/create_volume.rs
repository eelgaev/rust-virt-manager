use egui::{Ui, ViewportBuilder, ViewportId};

use crate::backend::BackendCommand;
use crate::state::AppState;

pub fn show_create_volume_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.create_volume.open {
        return;
    }

    let title = format!(
        "Create Volume in '{}'",
        state.create_volume.pool_name
    );

    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("create_volume"),
        ViewportBuilder::default()
            .with_title(&title)
            .with_inner_size([400.0, 250.0])
            .with_resizable(false),
        |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                state.create_volume.open = false;
            }
            egui::CentralPanel::default().show(ui.ctx(), |ui| {
                show_create_volume_content(ui, state);
            });
        },
    );
}

fn show_create_volume_content(ui: &mut Ui, state: &mut AppState) {
    ui.heading("New Volume");
    ui.separator();

    egui::Grid::new("create_vol_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut state.create_volume.name);
            ui.end_row();

            ui.label("Format:");
            egui::ComboBox::from_id_salt("vol_format")
                .selected_text(&state.create_volume.format)
                .show_ui(ui, |ui| {
                    for fmt in &["qcow2", "raw", "vmdk", "vpc", "qed"] {
                        ui.selectable_value(
                            &mut state.create_volume.format,
                            fmt.to_string(),
                            *fmt,
                        );
                    }
                });
            ui.end_row();

            ui.label("Capacity (GiB):");
            ui.add(egui::DragValue::new(&mut state.create_volume.capacity_gib).range(1..=65536));
            ui.end_row();
        });

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        let name_ok = !state.create_volume.name.trim().is_empty();

        if ui.add_enabled(name_ok, egui::Button::new("✅ Create")).clicked() {
            let uri = state.create_volume.uri.clone();
            let pool = state.create_volume.pool_name.clone();
            let name = state.create_volume.name.trim().to_string();
            let format = state.create_volume.format.clone();
            let cap_bytes = state.create_volume.capacity_gib * 1024 * 1024 * 1024;

            let vol_xml = format!(
                "<volume>\n  <name>{name}</name>\n  <capacity unit='bytes'>{cap_bytes}</capacity>\n  <target>\n    <format type='{format}'/>\n  </target>\n</volume>"
            );

            state.backend.send_to(
                &uri,
                BackendCommand::CreateVolume(uri.clone(), pool.clone(), vol_xml),
            );
            state.backend.send_to(
                &uri,
                BackendCommand::ListVolumes(uri.clone(), pool),
            );
            state.create_volume.open = false;
        }

        if ui.button("❌ Cancel").clicked() {
            state.create_volume.open = false;
        }
    });
}
