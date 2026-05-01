use egui::{RichText, Ui, ViewportBuilder, ViewportId};

use crate::backend::BackendCommand;
use crate::state::{AppState, ConnDetailsState};

pub fn show_conn_details_windows(ctx: &egui::Context, state: &mut AppState) {
    let keys: Vec<String> = state.conn_details.keys().cloned().collect();
    let mut to_close = Vec::new();

    for uri in &keys {
        let title = format!("{uri} - Connection Details");

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of(format!("conn_details_{uri}")),
            ViewportBuilder::default()
                .with_title(&title)
                .with_inner_size([750.0, 500.0]),
            |ui, _class| {
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    to_close.push(uri.clone());
                }
                egui::CentralPanel::default().show(ui.ctx(), |ui| {
                    show_conn_details_content(ui, state, uri);
                });
            },
        );
    }

    for key in to_close {
        state.conn_details.remove(&key);
    }
}

fn show_conn_details_content(ui: &mut Ui, state: &mut AppState, uri: &str) {
    ui.horizontal(|ui| {
        let tab = state.conn_details.get(uri).map_or(0, |c| c.active_tab);
        if ui.selectable_label(tab == 0, "📊 Overview").clicked() {
            state.conn_details.get_mut(uri).unwrap().active_tab = 0;
        }
        if ui.selectable_label(tab == 1, "💾 Storage").clicked() {
            state.conn_details.get_mut(uri).unwrap().active_tab = 1;
            state.backend.send_to(uri, BackendCommand::ListStoragePools(uri.to_string()));
        }
        if ui.selectable_label(tab == 2, "🌐 Networks").clicked() {
            state.conn_details.get_mut(uri).unwrap().active_tab = 2;
            state.backend.send_to(uri, BackendCommand::ListNetworks(uri.to_string()));
        }
    });

    ui.separator();

    let tab = state.conn_details.get(uri).map_or(0, |c| c.active_tab);
    match tab {
        0 => show_overview_tab(ui, state, uri),
        1 => show_storage_tab(ui, state, uri),
        2 => show_networks_tab(ui, state, uri),
        _ => {}
    }
}

fn show_overview_tab(ui: &mut Ui, state: &mut AppState, uri: &str) {
    if state.conn_details.get(uri).and_then(|c| c.host_info.as_ref()).is_none() {
        state.backend.send_to(uri, BackendCommand::GetHostInfo(uri.to_string()));
        ui.label("Loading host info...");
        return;
    }

    let info = state.conn_details[uri].host_info.as_ref().unwrap();
    ui.heading("Host Overview");
    ui.separator();

    egui::Grid::new("host_overview")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label("Hostname:");
            ui.label(&info.hostname);
            ui.end_row();

            ui.label("URI:");
            ui.label(uri);
            ui.end_row();

            ui.label("Architecture:");
            ui.label(&info.arch);
            ui.end_row();

            ui.label("CPU Model:");
            ui.label(&info.model);
            ui.end_row();

            ui.label("CPUs:");
            ui.label(format!("{}", info.cpus));
            ui.end_row();

            ui.label("CPU MHz:");
            ui.label(format!("{}", info.mhz));
            ui.end_row();

            ui.label("Memory:");
            let mem_gib = info.memory_kb as f64 / (1024.0 * 1024.0);
            ui.label(format!("{:.1} GiB", mem_gib));
            ui.end_row();

            ui.label("Libvirt Version:");
            let major = info.lib_version / 1_000_000;
            let minor = (info.lib_version / 1_000) % 1_000;
            let micro = info.lib_version % 1_000;
            ui.label(format!("{major}.{minor}.{micro}"));
            ui.end_row();
        });
}

fn show_storage_tab(ui: &mut Ui, state: &mut AppState, uri: &str) {
    ui.horizontal(|ui| {
        if ui.button("🔄 Refresh All").clicked() {
            state.backend.send_to(uri, BackendCommand::ListStoragePools(uri.to_string()));
        }
        if ui.button("➕ Create Pool").clicked() {
            state.create_pool.open = true;
            state.create_pool.uri = uri.to_string();
        }
    });
    ui.separator();

    let pools = state.conn_details.get(uri).map_or(vec![], |c| c.pools.clone());

    if pools.is_empty() {
        ui.label("No storage pools. Click 'Refresh All' to load.");
        return;
    }

    ui.columns(2, |cols| {
        egui::ScrollArea::vertical()
            .id_salt("pool_list")
            .show(&mut cols[0], |ui| {
                for pool in &pools {
                    let label = if pool.active {
                        format!("▶ {} ({:.1} GB)", pool.name, pool.capacity as f64 / 1e9)
                    } else {
                        format!("⏹ {} (inactive)", pool.name)
                    };
                    let is_selected = state.conn_details.get(uri)
                        .and_then(|c| c.selected_pool.as_deref())
                        == Some(&pool.name);
                    if ui.selectable_label(is_selected, &label).clicked() {
                        let cd = state.conn_details.get_mut(uri).unwrap();
                        cd.selected_pool = Some(pool.name.clone());
                        state.backend.send_to(
                            uri,
                            BackendCommand::ListVolumes(uri.to_string(), pool.name.clone()),
                        );
                    }
                }
            });

        let selected_pool = state.conn_details.get(uri).and_then(|c| c.selected_pool.clone());
        egui::ScrollArea::vertical()
            .id_salt("vol_list")
            .show(&mut cols[1], |ui| {
                if let Some(pool_name) = &selected_pool {
                    ui.heading(pool_name);

                    if let Some(pool) = pools.iter().find(|p| &p.name == pool_name) {
                        ui.label(format!(
                            "Capacity: {:.1} GB | Used: {:.1} GB | Free: {:.1} GB",
                            pool.capacity as f64 / 1e9,
                            pool.allocation as f64 / 1e9,
                            pool.available as f64 / 1e9,
                        ));
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            if pool.active {
                                if ui.button("⏹ Stop").clicked() {
                                    state.backend.send_to(
                                        uri,
                                        BackendCommand::StopStoragePool(uri.to_string(), pool_name.clone()),
                                    );
                                }
                            } else if ui.button("▶ Start").clicked() {
                                state.backend.send_to(
                                    uri,
                                    BackendCommand::StartStoragePool(uri.to_string(), pool_name.clone()),
                                );
                            }

                            if ui.button("🔄 Refresh").clicked() {
                                state.backend.send_to(
                                    uri,
                                    BackendCommand::RefreshStoragePool(uri.to_string(), pool_name.clone()),
                                );
                                state.backend.send_to(
                                    uri,
                                    BackendCommand::ListVolumes(uri.to_string(), pool_name.clone()),
                                );
                            }

                            if ui.button("🗑 Delete Pool").clicked() {
                                state.conn_details.get_mut(uri).unwrap().confirm_delete_pool =
                                    Some(pool_name.clone());
                            }

                            if pool.active {
                                if ui.button("➕ Create Volume").clicked() {
                                    state.create_volume.open = true;
                                    state.create_volume.uri = uri.to_string();
                                    state.create_volume.pool_name = pool_name.clone();
                                    state.create_volume.name.clear();
                                }
                            }
                        });

                        if let Some(ref confirm_pool) = state.conn_details.get(uri).unwrap().confirm_delete_pool.clone() {
                            if confirm_pool == pool_name {
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Delete this pool?").color(egui::Color32::YELLOW));
                                    if ui.button("Yes, delete").clicked() {
                                        state.backend.send_to(
                                            uri,
                                            BackendCommand::DeleteStoragePool(uri.to_string(), pool_name.clone()),
                                        );
                                        let cd = state.conn_details.get_mut(uri).unwrap();
                                        cd.selected_pool = None;
                                        cd.confirm_delete_pool = None;
                                    }
                                    if ui.button("Cancel").clicked() {
                                        state.conn_details.get_mut(uri).unwrap().confirm_delete_pool = None;
                                    }
                                });
                            }
                        }
                    }

                    ui.separator();

                    let vols = state.conn_details.get(uri)
                        .and_then(|c| c.volumes.get(pool_name))
                        .cloned()
                        .unwrap_or_default();

                    if vols.is_empty() {
                        ui.label("No volumes in this pool.");
                    } else {
                        let mut vol_to_delete = None;
                        egui::Grid::new("vol_grid")
                            .num_columns(5)
                            .spacing([8.0, 4.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label(RichText::new("Name").strong());
                                ui.label(RichText::new("Format").strong());
                                ui.label(RichText::new("Capacity").strong());
                                ui.label(RichText::new("Allocation").strong());
                                ui.label(RichText::new("").strong());
                                ui.end_row();

                                for vol in &vols {
                                    ui.label(&vol.name);
                                    ui.label(&vol.vol_type);
                                    ui.label(format_bytes(vol.capacity));
                                    ui.label(format_bytes(vol.allocation));
                                    if ui.small_button("🗑").on_hover_text("Delete volume").clicked() {
                                        vol_to_delete = Some(vol.name.clone());
                                    }
                                    ui.end_row();
                                }
                            });

                        if let Some(ref confirm_vol) = state.conn_details.get(uri).unwrap().confirm_delete_vol.clone() {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("Delete volume '{confirm_vol}'?"))
                                        .color(egui::Color32::YELLOW),
                                );
                                if ui.button("Yes, delete").clicked() {
                                    state.backend.send_to(
                                        uri,
                                        BackendCommand::DeleteVolume(
                                            uri.to_string(),
                                            pool_name.clone(),
                                            confirm_vol.clone(),
                                        ),
                                    );
                                    state.backend.send_to(
                                        uri,
                                        BackendCommand::ListVolumes(uri.to_string(), pool_name.clone()),
                                    );
                                    state.conn_details.get_mut(uri).unwrap().confirm_delete_vol = None;
                                }
                                if ui.button("Cancel").clicked() {
                                    state.conn_details.get_mut(uri).unwrap().confirm_delete_vol = None;
                                }
                            });
                        }

                        if let Some(name) = vol_to_delete {
                            state.conn_details.get_mut(uri).unwrap().confirm_delete_vol = Some(name);
                        }
                    }
                } else {
                    ui.label("Select a pool to view volumes.");
                }
            });
    });
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GiB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn show_networks_tab(ui: &mut Ui, state: &mut AppState, uri: &str) {
    ui.horizontal(|ui| {
        if ui.button("🔄 Refresh").clicked() {
            state.backend.send_to(uri, BackendCommand::ListNetworks(uri.to_string()));
        }
        if ui.button("➕ Create Network").clicked() {
            state.create_network.open = true;
            state.create_network.uri = uri.to_string();
        }
    });
    ui.separator();

    let networks = state.conn_details.get(uri).map_or(vec![], |c| c.networks.clone());

    if networks.is_empty() {
        ui.label("No virtual networks. Click 'Refresh' to load.");
        return;
    }

    ui.columns(2, |cols| {
        egui::ScrollArea::vertical()
            .id_salt("net_list")
            .show(&mut cols[0], |ui| {
                for net in &networks {
                    let label = if net.active {
                        format!("▶ {}", net.name)
                    } else {
                        format!("⏹ {} (inactive)", net.name)
                    };
                    let is_selected = state.conn_details.get(uri)
                        .and_then(|c| c.selected_network.as_deref())
                        == Some(&net.name);
                    if ui.selectable_label(is_selected, &label).clicked() {
                        state.conn_details.get_mut(uri).unwrap().selected_network =
                            Some(net.name.clone());
                    }
                }
            });

        let selected_net = state.conn_details.get(uri).and_then(|c| c.selected_network.clone());
        egui::ScrollArea::vertical()
            .id_salt("net_detail")
            .show(&mut cols[1], |ui| {
                if let Some(net_name) = &selected_net {
                    if let Some(net) = networks.iter().find(|n| &n.name == net_name) {
                        ui.heading(&net.name);
                        ui.separator();

                        egui::Grid::new("net_info")
                            .num_columns(2)
                            .spacing([12.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Active:");
                                ui.label(if net.active { "Yes" } else { "No" });
                                ui.end_row();

                                ui.label("Autostart:");
                                ui.label(if net.autostart { "Yes" } else { "No" });
                                ui.end_row();

                                if !net.bridge.is_empty() {
                                    ui.label("Bridge:");
                                    ui.label(&net.bridge);
                                    ui.end_row();
                                }

                                ui.label("UUID:");
                                ui.label(&net.uuid);
                                ui.end_row();
                            });

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if net.active {
                                if ui.button("⏹ Stop").clicked() {
                                    state.backend.send_to(
                                        uri,
                                        BackendCommand::StopNetwork(
                                            uri.to_string(),
                                            net_name.clone(),
                                        ),
                                    );
                                }
                            } else if ui.button("▶ Start").clicked() {
                                state.backend.send_to(
                                    uri,
                                    BackendCommand::StartNetwork(
                                        uri.to_string(),
                                        net_name.clone(),
                                    ),
                                );
                            }

                            if ui.button("🗑 Delete").clicked() {
                                state.backend.send_to(
                                    uri,
                                    BackendCommand::DeleteNetwork(
                                        uri.to_string(),
                                        net_name.clone(),
                                    ),
                                );
                                state.conn_details.get_mut(uri).unwrap().selected_network = None;
                            }
                        });
                    }
                } else {
                    ui.label("Select a network to view details.");
                }
            });
    });
}

pub fn open_conn_details(state: &mut AppState, uri: &str) {
    if !state.conn_details.contains_key(uri) {
        state.conn_details.insert(uri.to_string(), ConnDetailsState::new());
    }
}
