use eframe::App;

use crate::backend::BackendEvent;
use crate::state::{AppState, ConnectionState, GuiDomain};
use crate::views::{add_hardware, clone_vm, conn_details, create_network, create_pool, create_vm, create_volume, manager, vm_window, volume_browser};

pub struct VirtManagerApp {
    state: AppState,
}

impl VirtManagerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: AppState::new(),
        }
    }

    fn process_backend_events(&mut self) {
        let events = self.state.backend.drain_events();
        for event in events {
            match event {
                BackendEvent::ConnectionOpened(uri) => {
                    log::info!("Connected to {uri}");
                    if let Some(conn) = self.state.find_connection_mut(&uri) {
                        conn.state = ConnectionState::Active;
                    }
                }
                BackendEvent::ConnectionFailed(uri, err) => {
                    log::error!("Connection to {uri} failed: {err}");
                    if let Some(conn) = self.state.find_connection_mut(&uri) {
                        conn.state = ConnectionState::Disconnected;
                    }
                    self.state.set_error(format!("Connection to {uri} failed: {err}"));
                }
                BackendEvent::ConnectionClosed(uri) => {
                    log::info!("Disconnected from {uri}");
                    if let Some(conn) = self.state.find_connection_mut(&uri) {
                        conn.state = ConnectionState::Disconnected;
                        conn.domains.clear();
                    }
                }
                BackendEvent::DomainListUpdated(uri, summaries) => {
                    if let Some(conn) = self.state.find_connection_mut(&uri) {
                        let existing_names: Vec<String> =
                            conn.domains.keys().cloned().collect();
                        let new_names: Vec<String> =
                            summaries.iter().map(|s| s.name.clone()).collect();

                        for name in &existing_names {
                            if !new_names.contains(name) {
                                conn.domains.remove(name);
                            }
                        }

                        for summary in &summaries {
                            if let Some(domain) = conn.domains.get_mut(&summary.name) {
                                domain.state = summary.state;
                                domain.vcpus = summary.vcpus;
                                domain.memory_kib = summary.memory_kib;
                                domain.xml = summary.xml.clone();
                            } else {
                                conn.domains
                                    .insert(summary.name.clone(), GuiDomain::from_summary(summary));
                            }
                        }
                    }
                }
                BackendEvent::DomainStateChanged { uri, name, state } => {
                    if let Some(conn) = self.state.find_connection_mut(&uri) {
                        if let Some(domain) = conn.domains.get_mut(&name) {
                            domain.state = state;
                        }
                    }
                }
                BackendEvent::DomainStatsUpdated(uri, stats_list) => {
                    if let Some(conn) = self.state.find_connection_mut(&uri) {
                        for stats in &stats_list {
                            if let Some(domain) = conn.domains.get_mut(&stats.name) {
                                domain.stats.push_cpu(stats.cpu_percent);
                                domain.stats.push_memory(stats.memory_rss_kib as f64);
                            }
                        }
                    }
                }
                BackendEvent::DomainDefined(uri, name) => {
                    log::info!("[{uri}] Domain '{name}' defined");
                }
                BackendEvent::DomainUndefined(uri, name) => {
                    log::info!("[{uri}] Domain '{name}' undefined");
                    if let Some(conn) = self.state.find_connection_mut(&uri) {
                        conn.domains.remove(&name);
                    }
                }
                BackendEvent::HostInfoUpdated(uri, info) => {
                    if let Some(conn_state) = self.state.conn_details.get_mut(&uri) {
                        conn_state.host_info = Some(info);
                    }
                }
                BackendEvent::StoragePoolListUpdated(uri, pools) => {
                    if let Some(conn_state) = self.state.conn_details.get_mut(&uri) {
                        conn_state.pools = pools;
                    }
                }
                BackendEvent::VolumeListUpdated(uri, pool, vols) => {
                    if let Some(conn_state) = self.state.conn_details.get_mut(&uri) {
                        conn_state.volumes.insert(pool, vols);
                    }
                }
                BackendEvent::NetworkListUpdated(uri, nets) => {
                    if let Some(conn_state) = self.state.conn_details.get_mut(&uri) {
                        conn_state.networks = nets;
                    }
                }
                BackendEvent::SnapshotListUpdated(_uri, _domain, _snaps) => {
                    // Snapshots feature disabled for now
                }
                BackendEvent::MigrationComplete(uri, name) => {
                    log::info!("[{uri}] Migration of '{name}' complete");
                }
                BackendEvent::QemuCapsUpdated(_uri, arch, caps) => {
                    self.state.qemu_caps.insert(arch, caps);
                }
                BackendEvent::Error(uri, msg) => {
                    log::error!("[{uri}] {msg}");
                    self.state.set_error(msg);
                }
            }
        }
    }
}

impl App for VirtManagerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.process_backend_events();

        ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));

        egui::Panel::top("menu_bar").show_inside(ui, |ui| {
            manager::show_menu_bar(ui, &mut self.state);
        });

        egui::Panel::top("toolbar").show_inside(ui, |ui| {
            manager::show_toolbar(ui, &mut self.state);
        });

        if let Some((msg, when)) = &self.state.error_message {
            if when.elapsed() < std::time::Duration::from_secs(10) {
                egui::Panel::bottom("error_bar").show_inside(ui, |ui| {
                    ui.colored_label(egui::Color32::RED, msg);
                });
            } else {
                self.state.error_message = None;
            }
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            manager::show_vm_list(ui, &mut self.state);
        });

        manager::show_add_connection_window(ui.ctx(), &mut self.state);
        vm_window::show_vm_windows(ui.ctx(), &mut self.state);
        conn_details::show_conn_details_windows(ui.ctx(), &mut self.state);
        create_vm::show_create_vm_window(ui.ctx(), &mut self.state);
        add_hardware::show_add_hardware_window(ui.ctx(), &mut self.state);
        create_pool::show_create_pool_window(ui.ctx(), &mut self.state);
        create_network::show_create_network_window(ui.ctx(), &mut self.state);
        clone_vm::show_clone_vm_window(ui.ctx(), &mut self.state);
        create_volume::show_create_volume_window(ui.ctx(), &mut self.state);
        volume_browser::show_volume_browser_window(ui.ctx(), &mut self.state);
        manager::show_about_window(ui.ctx(), &mut self.state);
        manager::show_preferences_window(ui.ctx(), &mut self.state);
        manager::show_delete_vm_window(ui.ctx(), &mut self.state);
        manager::show_migrate_window(ui.ctx(), &mut self.state);
    }
}
