use egui::{Ui, ViewportBuilder, ViewportId};

use crate::backend::BackendCommand;
use crate::state::AppState;

const POOL_TYPES: [&str; 4] = ["dir", "logical", "netfs", "disk"];

pub struct CreatePoolState {
    pub open: bool,
    pub uri: String,
    pub name: String,
    pub pool_type: usize,
    pub target_path: String,
    pub source_host: String,
    pub source_path: String,
    pub source_dev: String,
}

impl Default for CreatePoolState {
    fn default() -> Self {
        Self {
            open: false,
            uri: String::new(),
            name: String::new(),
            pool_type: 0,
            target_path: "/var/lib/libvirt/images".into(),
            source_host: String::new(),
            source_path: String::new(),
            source_dev: String::new(),
        }
    }
}

pub fn show_create_pool_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.create_pool.open {
        return;
    }

    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("create_pool"),
        ViewportBuilder::default()
            .with_title("Create Storage Pool")
            .with_inner_size([450.0, 350.0]),
        |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                state.create_pool = CreatePoolState::default();
            }
            egui::CentralPanel::default().show(ui.ctx(), |ui| {
                show_pool_form(ui, state);

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("✅ Create").clicked() {
                        create_pool(state);
                        state.create_pool = CreatePoolState::default();
                    }
                    if ui.button("Cancel").clicked() {
                        state.create_pool = CreatePoolState::default();
                    }
                });
            });
        },
    );
}

fn show_pool_form(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Storage Pool Configuration");
    ui.add_space(8.0);

    egui::Grid::new("pool_form_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut state.create_pool.name);
            ui.end_row();

            ui.label("Type:");
            egui::ComboBox::from_id_salt("pool_type_combo")
                .selected_text(POOL_TYPES[state.create_pool.pool_type])
                .show_ui(ui, |ui| {
                    for (i, t) in POOL_TYPES.iter().enumerate() {
                        ui.selectable_value(&mut state.create_pool.pool_type, i, *t);
                    }
                });
            ui.end_row();

            ui.label("Target Path:");
            ui.text_edit_singleline(&mut state.create_pool.target_path);
            ui.end_row();

            let pt = state.create_pool.pool_type;
            if pt == 2 {
                ui.label("Source Host:");
                ui.text_edit_singleline(&mut state.create_pool.source_host);
                ui.end_row();

                ui.label("Source Path:");
                ui.text_edit_singleline(&mut state.create_pool.source_path);
                ui.end_row();
            }

            if pt == 1 || pt == 3 {
                ui.label("Source Device:");
                ui.text_edit_singleline(&mut state.create_pool.source_dev);
                ui.end_row();
            }
        });
}

fn create_pool(state: &mut AppState) {
    let p = &state.create_pool;
    let pt = POOL_TYPES[p.pool_type];

    let source_xml = match p.pool_type {
        2 => format!(
            "<source><host name='{}'/><dir path='{}'/></source>",
            p.source_host, p.source_path
        ),
        1 | 3 => format!(
            "<source><device path='{}'/></source>",
            p.source_dev
        ),
        _ => String::new(),
    };

    let xml = format!(
        "<pool type='{pt}'>\
           <name>{}</name>\
           {source_xml}\
           <target><path>{}</path></target>\
         </pool>",
        p.name, p.target_path,
    );

    let uri = p.uri.clone();
    state
        .backend
        .send_to(&uri, BackendCommand::CreateStoragePool(uri.clone(), xml));
}
