use egui::{Color32, RichText, Ui};
use egui_extras::{Column, TableBuilder};

use crate::backend::{BackendCommand, DomainAction};
use crate::state::{AppState, ConnectionState};
use crate::widgets::sparkline;

pub fn show_menu_bar(ui: &mut Ui, state: &mut AppState) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Add Connection...").clicked() {
                state.add_connection.open = true;
                ui.close();
            }
            if ui.button("New Virtual Machine").clicked() {
                state.create_vm.open = true;
                if let Some(uri) = state.selected_uri.clone() {
                    state.create_vm.uri = uri;
                }
                ui.close();
            }
            ui.separator();
            if ui.button("Quit").clicked() {
                std::process::exit(0);
            }
        });

        ui.menu_button("Edit", |ui| {
            if ui.button("Connection Details").clicked() {
                if let Some(uri) = state.selected_uri.clone() {
                    super::conn_details::open_conn_details(state, &uri);
                }
                ui.close();
            }
            if ui.button("Virtual Machine Details").clicked() {
                if let (Some(uri), Some(name)) = (state.selected_uri.clone(), state.selected_domain.clone()) {
                    super::vm_window::open_vm_window(state, &uri, &name);
                }
                ui.close();
            }
            if ui.button("Delete").clicked() {
                if let (Some(uri), Some(name)) = (&state.selected_uri, &state.selected_domain) {
                    state.delete_vm.open = true;
                    state.delete_vm.uri = uri.clone();
                    state.delete_vm.domain_name = name.clone();
                }
                ui.close();
            }
            if ui.button("Clone").clicked() {
                if let (Some(uri), Some(name)) = (&state.selected_uri, &state.selected_domain) {
                    state.clone_vm.open = true;
                    state.clone_vm.uri = uri.clone();
                    state.clone_vm.source_name = name.clone();
                    state.clone_vm.clone_name = format!("{name}-clone");
                    let disk_count = state
                        .connections
                        .iter()
                        .find(|c| c.uri == *uri)
                        .and_then(|c| c.domains.get(name))
                        .and_then(|d| {
                            crate::domain::Guest::from_xml(&d.xml).ok()
                        })
                        .and_then(|g| g.devices.map(|d| d.disks.len()))
                        .unwrap_or(0);
                    state.clone_vm.disk_strategies = vec![
                        crate::views::clone_vm::CloneDiskStrategy::Clone;
                        disk_count
                    ];
                }
                ui.close();
            }
            if ui.button("Migrate").clicked() {
                if let (Some(uri), Some(name)) = (&state.selected_uri, &state.selected_domain) {
                    state.migrate.open = true;
                    state.migrate.uri = uri.clone();
                    state.migrate.domain_name = name.clone();
                }
                ui.close();
            }
            ui.separator();
            if ui.button("Preferences").clicked() {
                state.show_preferences = true;
                ui.close();
            }
        });

        ui.menu_button("View", |ui| {
            ui.menu_button("Graph", |ui| {
                ui.checkbox(&mut state.config.show_guest_cpu, "Guest CPU Usage");
                ui.checkbox(&mut state.config.show_host_cpu, "Host CPU Usage");
                ui.checkbox(&mut state.config.show_memory, "Memory Usage");
                ui.checkbox(&mut state.config.show_disk_io, "Disk I/O");
                ui.checkbox(&mut state.config.show_network_io, "Network I/O");
            });
        });

        ui.menu_button("Help", |ui| {
            if ui.button("About").clicked() {
                state.show_about = true;
                ui.close();
            }
        });
    });
}

pub fn show_toolbar(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("➕ New").clicked() {
            state.create_vm.open = true;
            if let Some(uri) = state.selected_uri.clone() {
                state.create_vm.uri = uri;
            }
        }

        ui.separator();

        if ui.button("📋 Open").clicked() {
            if let (Some(uri), Some(name)) = (state.selected_uri.clone(), state.selected_domain.clone()) {
                super::vm_window::open_vm_window(state, &uri, &name);
            }
        }

        let has_selection = state.selected_domain.is_some();
        let domain_state = state
            .selected_domain_info()
            .map(|(_, d)| d.state);

        let can_start = domain_state
            .is_some_and(|s| !s.is_active());
        let can_pause = domain_state
            .is_some_and(|s| s == crate::domain::DomainState::Running);
        let can_shutdown = domain_state
            .is_some_and(|s| s.is_active());

        if ui.add_enabled(has_selection && can_start, egui::Button::new("▶ Run")).clicked() {
            send_domain_action(state, DomainAction::Start);
        }

        if ui.add_enabled(has_selection && can_pause, egui::Button::new("⏸ Pause")).clicked() {
            let is_paused = domain_state == Some(crate::domain::DomainState::Paused);
            if is_paused {
                send_domain_action(state, DomainAction::Resume);
            } else {
                send_domain_action(state, DomainAction::Pause);
            }
        }

        let shutdown_response = ui.add_enabled(
            has_selection && can_shutdown,
            egui::Button::new("⏻ Shut Down"),
        );

        if shutdown_response.clicked() {
            send_domain_action(state, DomainAction::Shutdown);
        }

        shutdown_response.context_menu(|ui| {
            if ui.button("🔄 Reboot").clicked() {
                send_domain_action(state, DomainAction::Reboot);
                ui.close();
            }
            if ui.button("⏻ Shut Down").clicked() {
                send_domain_action(state, DomainAction::Shutdown);
                ui.close();
            }
            if ui.button("⚡ Force Off").clicked() {
                send_domain_action(state, DomainAction::ForceOff);
                ui.close();
            }
        });
    });
}

pub fn show_vm_list(ui: &mut Ui, state: &mut AppState) {
    let mut action: Option<(String, String, DomainAction)> = None;

    let show_cpu = state.config.show_guest_cpu;
    let show_mem = state.config.show_memory;
    let show_disk = state.config.show_disk_io;
    let show_net = state.config.show_network_io;

    let mut builder = TableBuilder::new(ui)
        .striped(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::remainder().at_least(150.0));
    if show_cpu { builder = builder.column(Column::auto().at_least(70.0)); }
    if show_mem { builder = builder.column(Column::auto().at_least(80.0)); }
    if show_disk { builder = builder.column(Column::auto().at_least(70.0)); }
    if show_net { builder = builder.column(Column::auto().at_least(70.0)); }

    builder
        .header(20.0, |mut header| {
            header.col(|ui| { ui.strong("Name"); });
            if show_cpu { header.col(|ui| { ui.strong("CPU"); }); }
            if show_mem { header.col(|ui| { ui.strong("Memory"); }); }
            if show_disk { header.col(|ui| { ui.strong("Disk I/O"); }); }
            if show_net { header.col(|ui| { ui.strong("Network"); }); }
        })
        .body(|mut body| {
            let conn_count = state.connections.len();
            for conn_idx in 0..conn_count {
                let conn = &state.connections[conn_idx];
                let uri = conn.uri.clone();
                let conn_state = conn.state;
                let expanded = conn.expanded;

                let friendly_name = crate::uri::LibvirtUri::parse(&uri)
                    .map(|u| u.display_name())
                    .unwrap_or_else(|_| uri.clone());

                body.row(22.0, |mut row| {
                    row.col(|ui| {
                        let arrow = if expanded { "▼" } else { "▶" };
                        let label_text = match conn_state {
                            ConnectionState::Active => {
                                RichText::new(format!("{arrow} {friendly_name}")).strong()
                            }
                            ConnectionState::Connecting => {
                                RichText::new(format!("{arrow} {friendly_name} (connecting...)")).weak()
                            }
                            ConnectionState::Disconnected => {
                                RichText::new(format!("{arrow} {friendly_name} (disconnected)")).weak()
                            }
                        };

                        let is_selected = state.selected_uri.as_deref() == Some(&uri)
                            && state.selected_domain.is_none();
                        let response = ui.selectable_label(is_selected, label_text);

                        if response.clicked() {
                            state.selected_uri = Some(uri.clone());
                            state.selected_domain = None;
                            state.connections[conn_idx].expanded = !expanded;
                        }

                        response.context_menu(|ui| {
                            match conn_state {
                                ConnectionState::Active => {
                                    if ui.button("New Virtual Machine").clicked() {
                                        state.create_vm.open = true;
                                        state.create_vm.uri = uri.clone();
                                        ui.close();
                                    }
                                    if ui.button("Connection Details").clicked() {
                                        super::conn_details::open_conn_details(state, &uri);
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui.button("Disconnect").clicked() {
                                        state.backend.stop_connection(&uri);
                                        state.connections[conn_idx].state = ConnectionState::Disconnected;
                                        state.connections[conn_idx].domains.clear();
                                        ui.close();
                                    }
                                }
                                ConnectionState::Disconnected => {
                                    if ui.button("Connect").clicked() {
                                        state.connections[conn_idx].state = ConnectionState::Connecting;
                                        state.backend.start_connection(uri.clone());
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui.button("Delete").clicked() {
                                        state.config.saved_uris.retain(|u| u != &uri);
                                        state.save_config();
                                        state.connections.retain(|c| c.uri != uri);
                                        ui.close();
                                    }
                                }
                                _ => {}
                            }
                        });
                    });

                    if show_cpu { row.col(|_ui| {}); }
                    if show_mem { row.col(|_ui| {}); }
                    if show_disk { row.col(|_ui| {}); }
                    if show_net { row.col(|_ui| {}); }
                });

                if !expanded {
                    continue;
                }

                let conn = &state.connections[conn_idx];
                let mut sorted_names: Vec<String> = conn.domains.keys().cloned().collect();
                sorted_names.sort();

                let domain_infos: Vec<_> = sorted_names.iter().map(|name| {
                    let d = &conn.domains[name];
                    (
                        name.clone(),
                        d.state,
                        d.memory_display(),
                        d.stats.cpu.clone(),
                        d.stats.disk_rd.clone(),
                        d.stats.net_rx.clone(),
                    )
                }).collect();

                for (domain_name, dom_state, mem_display, cpu_stats, disk_stats, net_stats) in &domain_infos {
                    let is_selected = state.selected_uri.as_deref() == Some(uri.as_str())
                        && state.selected_domain.as_deref() == Some(domain_name.as_str());

                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            let state_icon = match dom_state {
                                crate::domain::DomainState::Running => "▶",
                                crate::domain::DomainState::Paused => "⏸",
                                crate::domain::DomainState::Shutoff => "⏹",
                                _ => "?",
                            };

                            let label_text = format!("    {state_icon} {domain_name}");
                            let response = ui.selectable_label(is_selected, &label_text);

                            if response.clicked() {
                                state.selected_uri = Some(uri.clone());
                                state.selected_domain = Some(domain_name.clone());
                            }
                            if response.double_clicked() {
                                super::vm_window::open_vm_window(state, &uri, domain_name);
                            }

                            response.context_menu(|ui| {
                                if ui.button("Open").clicked() {
                                    super::vm_window::open_vm_window(state, &uri, domain_name);
                                    ui.close();
                                }
                                ui.separator();

                                if dom_state.is_active() {
                                    if *dom_state == crate::domain::DomainState::Running {
                                        if ui.button("Pause").clicked() {
                                            action = Some((uri.clone(), domain_name.clone(), DomainAction::Pause));
                                            ui.close();
                                        }
                                    }
                                    if *dom_state == crate::domain::DomainState::Paused {
                                        if ui.button("Resume").clicked() {
                                            action = Some((uri.clone(), domain_name.clone(), DomainAction::Resume));
                                            ui.close();
                                        }
                                    }
                                    if ui.button("Shut Down").clicked() {
                                        action = Some((uri.clone(), domain_name.clone(), DomainAction::Shutdown));
                                        ui.close();
                                    }
                                    if ui.button("Reboot").clicked() {
                                        action = Some((uri.clone(), domain_name.clone(), DomainAction::Reboot));
                                        ui.close();
                                    }
                                    if ui.button("Force Off").clicked() {
                                        action = Some((uri.clone(), domain_name.clone(), DomainAction::ForceOff));
                                        ui.close();
                                    }
                                } else {
                                    if ui.button("Start").clicked() {
                                        action = Some((uri.clone(), domain_name.clone(), DomainAction::Start));
                                        ui.close();
                                    }
                                }

                                ui.separator();
                                if ui.button("Clone").clicked() {
                                    state.clone_vm.open = true;
                                    state.clone_vm.uri = uri.clone();
                                    state.clone_vm.source_name = domain_name.clone();
                                    state.clone_vm.clone_name = format!("{domain_name}-clone");
                                    ui.close();
                                }
                                if ui.button("Migrate").clicked() {
                                    state.migrate.open = true;
                                    state.migrate.uri = uri.clone();
                                    state.migrate.domain_name = domain_name.clone();
                                    ui.close();
                                }
                                if ui.button("Delete").clicked() {
                                    state.delete_vm.open = true;
                                    state.delete_vm.uri = uri.clone();
                                    state.delete_vm.domain_name = domain_name.clone();
                                    ui.close();
                                }
                            });
                        });

                        if show_cpu {
                            row.col(|ui| {
                                if dom_state.is_active() {
                                    sparkline::sparkline(ui, cpu_stats, 100.0, Color32::from_rgb(0, 150, 0));
                                }
                            });
                        }

                        if show_mem {
                            row.col(|ui| {
                                ui.label(mem_display);
                            });
                        }

                        if show_disk {
                            row.col(|ui| {
                                if dom_state.is_active() {
                                    sparkline::sparkline(ui, disk_stats, 1.0, Color32::from_rgb(0, 100, 200));
                                }
                            });
                        }

                        if show_net {
                            row.col(|ui| {
                                if dom_state.is_active() {
                                    sparkline::sparkline(ui, net_stats, 1.0, Color32::from_rgb(200, 100, 0));
                                }
                            });
                        }
                    });
                }
            }
        });

    if let Some((uri, name, act)) = action {
        state.backend.send_to(&uri, BackendCommand::DomainAction(uri.clone(), name, act));
    }
}

pub fn show_add_connection_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.add_connection.open {
        return;
    }

    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("add_connection"),
        egui::ViewportBuilder::default()
            .with_title("Add Connection")
            .with_inner_size([450.0, 350.0]),
        |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                state.add_connection.reset();
            }
            egui::CentralPanel::default().show(ui.ctx(), |ui| {
                egui::Grid::new("add_conn_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Hypervisor:");
                        let selected_label = state.add_connection.hypervisor.label();
                        egui::ComboBox::from_id_salt("hypervisor_combo")
                            .selected_text(selected_label)
                            .show_ui(ui, |ui| {
                                for &hv in crate::state::Hypervisor::ALL {
                                    if hv == crate::state::Hypervisor::CustomUri {
                                        ui.separator();
                                    }
                                    let was = state.add_connection.hypervisor;
                                    ui.selectable_value(
                                        &mut state.add_connection.hypervisor,
                                        hv,
                                        hv.label(),
                                    );
                                    if was != state.add_connection.hypervisor {
                                        if !state.add_connection.hypervisor.supports_remote() {
                                            state.add_connection.connect_remote = false;
                                        }
                                    }
                                }
                            });
                        ui.end_row();

                        if state.add_connection.hypervisor == crate::state::Hypervisor::QemuSession {
                            ui.label("");
                            ui.label(
                                RichText::new(
                                    "⚠ QEMU usermode session is not the default.\n\
                                     Pre-existing QEMU/KVM guests will not be\n\
                                     available. Networking options are limited.",
                                )
                                .small()
                                .weak(),
                            );
                            ui.end_row();
                        }

                        if state.add_connection.hypervisor.supports_remote() {
                            ui.label("");
                            let prev_remote = state.add_connection.connect_remote;
                            ui.checkbox(
                                &mut state.add_connection.connect_remote,
                                "Connect to remote host over SSH",
                            );
                            if state.add_connection.connect_remote && !prev_remote {
                                if state.add_connection.username.is_empty() {
                                    state.add_connection.username = "root".to_string();
                                }
                                state.add_connection.autoconnect = false;
                            }
                            if !state.add_connection.connect_remote && prev_remote {
                                state.add_connection.autoconnect = true;
                            }
                            ui.end_row();

                            if state.add_connection.connect_remote {
                                ui.label("    Username:");
                                ui.text_edit_singleline(&mut state.add_connection.username);
                                ui.end_row();

                                ui.label("    Hostname:");
                                ui.text_edit_singleline(&mut state.add_connection.hostname);
                                ui.end_row();
                            }
                        }

                        ui.separator();
                        ui.separator();
                        ui.end_row();

                        ui.label("Autoconnect:");
                        ui.checkbox(&mut state.add_connection.autoconnect, "");
                        ui.end_row();

                        if state.add_connection.hypervisor == crate::state::Hypervisor::CustomUri {
                            ui.label("Custom URI:");
                            ui.text_edit_singleline(&mut state.add_connection.custom_uri);
                            ui.end_row();
                        } else {
                            ui.label("Generated URI:");
                            let uri = state.add_connection.generated_uri();
                            ui.label(&uri);
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        state.add_connection.reset();
                    }

                    if ui.button("Connect").clicked() {
                        let uri = state.add_connection.generated_uri();

                        if state.add_connection.connect_remote
                            && state.add_connection.hostname.is_empty()
                        {
                            state.set_error("A hostname is required for remote connections.".into());
                            return;
                        }

                        if !uri.is_empty() && !state.connections.iter().any(|c| c.uri == uri) {
                            let mut conn = crate::state::GuiConnection::new(uri.clone());
                            conn.state = ConnectionState::Connecting;
                            state.connections.push(conn);
                            state.backend.start_connection(uri.clone());

                            if state.add_connection.autoconnect
                                && !state.config.saved_uris.contains(&uri)
                            {
                                state.config.saved_uris.push(uri);
                                state.save_config();
                            }
                        }
                        state.add_connection.reset();
                    }
                });
            });
        },
    );
}

fn send_domain_action(state: &mut AppState, action: DomainAction) {
    if let (Some(uri), Some(name)) = (&state.selected_uri, &state.selected_domain) {
        state.backend.send_to(
            uri,
            BackendCommand::DomainAction(uri.clone(), name.clone(), action),
        );
    }
}

pub fn show_about_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_about {
        return;
    }
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("about"),
        egui::ViewportBuilder::default()
            .with_title("About")
            .with_inner_size([350.0, 200.0]),
        |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                state.show_about = false;
            }
            egui::CentralPanel::default().show(ui.ctx(), |ui| {
                ui.heading("Virtual Machine Manager");
                ui.label("A fast, modern virt-manager replacement built in Rust with egui.");
                ui.add_space(4.0);
                ui.label("Version: 0.1.0");
                ui.label("License: GPL-2.0-or-later");
                ui.add_space(4.0);
                ui.label("Linux only. Manages KVM/QEMU VMs via libvirt.");
            });
        },
    );
}

pub fn show_preferences_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_preferences {
        return;
    }
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("preferences"),
        egui::ViewportBuilder::default()
            .with_title("Preferences")
            .with_inner_size([400.0, 350.0]),
        |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                state.show_preferences = false;
            }
            egui::CentralPanel::default().show(ui.ctx(), |ui| {
                ui.heading("General");
                ui.checkbox(&mut state.config.auto_connect, "Auto-connect on startup");
                ui.add_space(8.0);

                ui.heading("Polling");
                ui.horizontal(|ui| {
                    ui.label("Stats update interval (seconds):");
                    let mut interval = state.config.stats_update_interval_secs as i32;
                    ui.add(egui::DragValue::new(&mut interval).range(1..=60));
                    state.config.stats_update_interval_secs = interval.max(1) as u64;
                });
                ui.add_space(8.0);

                ui.heading("VM List Columns");
                ui.checkbox(&mut state.config.show_guest_cpu, "Show Guest CPU Usage");
                ui.checkbox(&mut state.config.show_host_cpu, "Show Host CPU Usage");
                ui.checkbox(&mut state.config.show_memory, "Show Memory Usage");
                ui.checkbox(&mut state.config.show_disk_io, "Show Disk I/O");
                ui.checkbox(&mut state.config.show_network_io, "Show Network I/O");

                ui.add_space(8.0);
                if ui.button("💾 Save").clicked() {
                    state.save_config();
                }
            });
        },
    );
}

pub fn show_delete_vm_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.delete_vm.open {
        return;
    }
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("delete_vm"),
        egui::ViewportBuilder::default()
            .with_title("Delete Virtual Machine")
            .with_inner_size([400.0, 200.0]),
        |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                state.delete_vm.open = false;
            }
            egui::CentralPanel::default().show(ui.ctx(), |ui| {
                ui.label(format!(
                    "Are you sure you want to delete '{}'?",
                    state.delete_vm.domain_name
                ));
                ui.add_space(8.0);

                for (path, checked) in &mut state.delete_vm.delete_storage {
                    ui.checkbox(checked, format!("Delete storage: {path}"));
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        state.delete_vm.open = false;
                    }
                    if ui
                        .button(RichText::new("🗑 Delete").color(Color32::RED))
                        .clicked()
                    {
                        let uri = state.delete_vm.uri.clone();
                        let name = state.delete_vm.domain_name.clone();
                        state.backend.send_to(
                            &uri,
                            BackendCommand::UndefineDomain(uri.clone(), name),
                        );
                        state.delete_vm.open = false;
                    }
                });
            });
        },
    );
}

pub fn show_migrate_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.migrate.open {
        return;
    }
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("migrate_vm"),
        egui::ViewportBuilder::default()
            .with_title("Migrate Virtual Machine")
            .with_inner_size([450.0, 250.0]),
        |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                state.migrate.open = false;
            }
            egui::CentralPanel::default().show(ui.ctx(), |ui| {
                ui.label(format!("Migrate '{}'", state.migrate.domain_name));
                ui.add_space(8.0);

                egui::Grid::new("migrate_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Destination URI:");
                        let uris: Vec<String> = state
                            .connections
                            .iter()
                            .filter(|c| c.uri != state.migrate.uri)
                            .map(|c| c.uri.clone())
                            .collect();
                        egui::ComboBox::from_id_salt("migrate_dest")
                            .selected_text(&state.migrate.dest_uri)
                            .show_ui(ui, |ui| {
                                for u in &uris {
                                    ui.selectable_value(&mut state.migrate.dest_uri, u.clone(), u);
                                }
                            });
                        ui.end_row();

                        ui.label("Live:");
                        ui.checkbox(&mut state.migrate.live, "");
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        state.migrate.open = false;
                    }
                    if ui.button("🚀 Migrate").clicked() {
                        let flags: u32 = if state.migrate.live { 1 } else { 0 };
                        state.backend.send_to(
                            &state.migrate.uri,
                            BackendCommand::MigrateDomain(
                                state.migrate.uri.clone(),
                                state.migrate.domain_name.clone(),
                                state.migrate.dest_uri.clone(),
                                flags,
                            ),
                        );
                        state.migrate.open = false;
                    }
                });
            });
        },
    );
}
