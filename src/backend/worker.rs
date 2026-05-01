use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use virt::domain::Domain;
use virt::domain_snapshot::DomainSnapshot;
use virt::network::Network;
use virt::storage_pool::StoragePool;
use virt::storage_vol::StorageVol;

use crate::connection::LibvirtConnection;
use crate::domain::DomainState;
use crate::uri::LibvirtUri;

use super::{
    BackendCommand, BackendEvent, DomainAction, DomainStats, DomainSummary, HostInfo,
    NetworkSummary, SnapshotSummary, StoragePoolSummary, VolumeSummary,
};

struct ConnectionWorker {
    uri: String,
    conn: LibvirtConnection,
    event_tx: Sender<BackendEvent>,
    prev_cpu_times: HashMap<String, (Instant, u64)>,
}

impl ConnectionWorker {
    fn new(uri: String, event_tx: Sender<BackendEvent>) -> Self {
        let parsed = LibvirtUri::parse(&uri).expect("URI was already validated");
        Self {
            uri,
            conn: LibvirtConnection::new(parsed),
            event_tx,
            prev_cpu_times: HashMap::new(),
        }
    }

    fn connect(&mut self) -> bool {
        match self.conn.open() {
            Ok(()) => {
                let _ = self.event_tx.send(BackendEvent::ConnectionOpened(self.uri.clone()));
                true
            }
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::ConnectionFailed(
                    self.uri.clone(),
                    e.to_string(),
                ));
                false
            }
        }
    }

    fn poll_domains(&mut self) {
        let domains = match self.conn.list_domains() {
            Ok(d) => d,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Failed to list domains: {e}"),
                ));
                return;
            }
        };

        let now = Instant::now();
        let mut summaries = Vec::with_capacity(domains.len());
        let mut stats = Vec::with_capacity(domains.len());

        for domain in &domains {
            if let Some((summary, domain_stats)) = self.collect_domain_info(domain, now) {
                summaries.push(summary);
                if let Some(s) = domain_stats {
                    stats.push(s);
                }
            }
        }

        let _ = self.event_tx.send(BackendEvent::DomainListUpdated(
            self.uri.clone(),
            summaries,
        ));

        if !stats.is_empty() {
            let _ = self.event_tx.send(BackendEvent::DomainStatsUpdated(
                self.uri.clone(),
                stats,
            ));
        }
    }

    fn collect_domain_info(
        &mut self,
        domain: &Domain,
        now: Instant,
    ) -> Option<(DomainSummary, Option<DomainStats>)> {
        let name = domain.get_name().ok()?;
        let uuid = domain.get_uuid_string().ok()?;
        let info = domain.get_info().ok()?;
        let state = DomainState::from_libvirt(info.state as u32);
        let xml = domain.get_xml_desc(0).unwrap_or_default();

        let summary = DomainSummary {
            name: name.clone(),
            uuid,
            state,
            vcpus: info.nr_virt_cpu as u32,
            memory_kib: info.memory,
            xml,
        };

        let domain_stats = if state.is_active() {
            let cpu_time = info.cpu_time;
            let cpu_percent = if let Some((prev_time, prev_cpu)) =
                self.prev_cpu_times.get(&name)
            {
                let dt = now.duration_since(*prev_time).as_secs_f64();
                if dt > 0.0 && info.nr_virt_cpu > 0 {
                    let dcpu = (cpu_time - prev_cpu) as f64;
                    (dcpu / (dt * 1_000_000_000.0 * info.nr_virt_cpu as f64)) * 100.0
                } else {
                    0.0
                }
            } else {
                0.0
            };

            self.prev_cpu_times.insert(name.clone(), (now, cpu_time));

            Some(DomainStats {
                name: name.clone(),
                cpu_time_ns: cpu_time,
                cpu_percent: cpu_percent.clamp(0.0, 100.0),
                memory_rss_kib: info.memory,
                ..Default::default()
            })
        } else {
            self.prev_cpu_times.remove(&name);
            None
        };

        Some((summary, domain_stats))
    }

    fn handle_domain_action(&self, domain_name: &str, action: DomainAction) {
        let conn = match self.conn.conn() {
            Ok(c) => c,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    e.to_string(),
                ));
                return;
            }
        };

        let domain = match Domain::lookup_by_name(conn, domain_name) {
            Ok(d) => d,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Domain '{domain_name}' not found: {e}"),
                ));
                return;
            }
        };

        let result: Result<(), virt::error::Error> = match action {
            DomainAction::Start => domain.create().map(|_| ()),
            DomainAction::Shutdown => domain.shutdown().map(|_| ()),
            DomainAction::ForceOff => domain.destroy().map(|_| ()),
            DomainAction::Pause => domain.suspend().map(|_| ()),
            DomainAction::Resume => domain.resume().map(|_| ()),
            DomainAction::Reboot => domain.reboot(0).map(|_| ()),
        };

        if let Err(e) = result {
            let _ = self.event_tx.send(BackendEvent::Error(
                self.uri.clone(),
                format!("Action on '{domain_name}' failed: {e}"),
            ));
        }
    }

    fn handle_define_xml(&self, xml: &str) {
        let conn = match self.conn.conn() {
            Ok(c) => c,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(self.uri.clone(), e.to_string()));
                return;
            }
        };
        match Domain::define_xml(conn, xml) {
            Ok(d) => {
                let name = d.get_name().unwrap_or_default();
                let _ = self.event_tx.send(BackendEvent::DomainDefined(self.uri.clone(), name));
            }
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Define XML failed: {e}"),
                ));
            }
        }
    }

    fn handle_undefine_domain(&self, name: &str) {
        let conn = match self.conn.conn() {
            Ok(c) => c,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(self.uri.clone(), e.to_string()));
                return;
            }
        };
        let domain = match Domain::lookup_by_name(conn, name) {
            Ok(d) => d,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Domain '{name}' not found: {e}"),
                ));
                return;
            }
        };
        match domain.undefine() {
            Ok(_) => {
                let _ = self.event_tx.send(BackendEvent::DomainUndefined(
                    self.uri.clone(),
                    name.to_string(),
                ));
            }
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Undefine '{name}' failed: {e}"),
                ));
            }
        }
    }

    fn handle_get_host_info(&self) {
        let conn = match self.conn.conn() {
            Ok(c) => c,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(self.uri.clone(), e.to_string()));
                return;
            }
        };
        let hostname = conn.get_hostname().unwrap_or_default();
        let info = match conn.get_node_info() {
            Ok(i) => i,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Get host info failed: {e}"),
                ));
                return;
            }
        };
        let lib_version = conn.get_lib_version().unwrap_or(0) as u32;
        let host_info = HostInfo {
            hostname,
            arch: info.model.clone(),
            memory_kb: info.memory,
            cpus: info.cpus,
            mhz: info.mhz,
            model: info.model,
            lib_version,
        };
        let _ = self.event_tx.send(BackendEvent::HostInfoUpdated(self.uri.clone(), host_info));
    }

    fn handle_list_pools(&self) {
        let pools = match self.conn.list_storage_pools() {
            Ok(p) => p,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("List pools failed: {e}"),
                ));
                return;
            }
        };
        let mut summaries = Vec::new();
        for pool in &pools {
            if let Ok(name) = pool.get_name() {
                let uuid = pool.get_uuid_string().unwrap_or_default();
                let info = pool.get_info().ok();
                let active = pool.is_active().unwrap_or(false);
                let autostart = pool.get_autostart().unwrap_or(false);
                let xml = pool.get_xml_desc(0).unwrap_or_default();
                summaries.push(StoragePoolSummary {
                    name,
                    uuid,
                    active,
                    autostart,
                    capacity: info.as_ref().map_or(0, |i| i.capacity),
                    allocation: info.as_ref().map_or(0, |i| i.allocation),
                    available: info.as_ref().map_or(0, |i| i.available),
                    xml,
                });
            }
        }
        let _ = self.event_tx.send(BackendEvent::StoragePoolListUpdated(
            self.uri.clone(),
            summaries,
        ));
    }

    fn handle_storage_pool_action(&self, pool_name: &str, action: &str, xml: Option<&str>) {
        let conn = match self.conn.conn() {
            Ok(c) => c,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(self.uri.clone(), e.to_string()));
                return;
            }
        };
        let result: Result<(), String> = match action {
            "create" => {
                StoragePool::define_xml(conn, xml.unwrap_or(""), 0)
                    .map(|p| { let _ = p.build(0); let _ = p.create(0); })
                    .map_err(|e| e.to_string())
            }
            "delete" => {
                StoragePool::lookup_by_name(conn, pool_name)
                    .and_then(|p| { let _ = p.destroy(); p.undefine() })
                    .map_err(|e| e.to_string())
            }
            "start" => {
                StoragePool::lookup_by_name(conn, pool_name)
                    .and_then(|p| p.create(0).map(|_| ()))
                    .map_err(|e| e.to_string())
            }
            "stop" => {
                StoragePool::lookup_by_name(conn, pool_name)
                    .and_then(|p| p.destroy().map(|_| ()))
                    .map_err(|e| e.to_string())
            }
            "refresh" => {
                StoragePool::lookup_by_name(conn, pool_name)
                    .and_then(|p| p.refresh(0).map(|_| ()))
                    .map_err(|e| e.to_string())
            }
            _ => Err(format!("Unknown pool action: {action}")),
        };
        if let Err(e) = result {
            let _ = self.event_tx.send(BackendEvent::Error(
                self.uri.clone(),
                format!("Pool '{pool_name}' {action} failed: {e}"),
            ));
        }
        self.handle_list_pools();
    }

    fn handle_list_volumes(&self, pool_name: &str) {
        let conn = match self.conn.conn() {
            Ok(c) => c,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(self.uri.clone(), e.to_string()));
                return;
            }
        };
        let pool = match StoragePool::lookup_by_name(conn, pool_name) {
            Ok(p) => p,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Pool '{pool_name}' not found: {e}"),
                ));
                return;
            }
        };
        let _ = pool.refresh(0);
        let vols = match pool.list_all_volumes(0) {
            Ok(v) => v,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("List volumes failed: {e}"),
                ));
                return;
            }
        };
        let mut summaries = Vec::new();
        for vol in &vols {
            if let Ok(name) = vol.get_name() {
                let path = vol.get_path().unwrap_or_default();
                let info = vol.get_info().ok();
                summaries.push(VolumeSummary {
                    name,
                    path,
                    capacity: info.as_ref().map_or(0, |i| i.capacity),
                    allocation: info.as_ref().map_or(0, |i| i.allocation),
                    vol_type: info.as_ref().map_or("unknown".into(), |i| format!("{}", i.kind)),
                });
            }
        }
        let _ = self.event_tx.send(BackendEvent::VolumeListUpdated(
            self.uri.clone(),
            pool_name.to_string(),
            summaries,
        ));
    }

    fn handle_volume_action(&self, pool_name: &str, action: &str, xml_or_name: &str) {
        let conn = match self.conn.conn() {
            Ok(c) => c,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(self.uri.clone(), e.to_string()));
                return;
            }
        };
        let pool = match StoragePool::lookup_by_name(conn, pool_name) {
            Ok(p) => p,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Pool '{pool_name}' not found: {e}"),
                ));
                return;
            }
        };
        let result: Result<(), String> = match action {
            "create" => {
                StorageVol::create_xml(&pool, xml_or_name, 0)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
            "delete" => {
                StorageVol::lookup_by_name(&pool, xml_or_name)
                    .and_then(|v| v.delete(0))
                    .map_err(|e| e.to_string())
            }
            _ => Err(format!("Unknown volume action: {action}")),
        };
        if let Err(e) = result {
            let _ = self.event_tx.send(BackendEvent::Error(
                self.uri.clone(),
                format!("Volume '{xml_or_name}' {action} failed: {e}"),
            ));
        }
        self.handle_list_volumes(pool_name);
    }

    fn handle_list_networks(&self) {
        let networks = match self.conn.list_networks() {
            Ok(n) => n,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("List networks failed: {e}"),
                ));
                return;
            }
        };
        let mut summaries = Vec::new();
        for net in &networks {
            if let Ok(name) = net.get_name() {
                let uuid = net.get_uuid_string().unwrap_or_default();
                let active = net.is_active().unwrap_or(false);
                let autostart = net.get_autostart().unwrap_or(false);
                let bridge = net.get_bridge_name().unwrap_or_default();
                let xml = net.get_xml_desc(0).unwrap_or_default();
                summaries.push(NetworkSummary {
                    name,
                    uuid,
                    active,
                    autostart,
                    bridge,
                    xml,
                });
            }
        }
        let _ = self.event_tx.send(BackendEvent::NetworkListUpdated(
            self.uri.clone(),
            summaries,
        ));
    }

    fn handle_network_action(&self, net_name: &str, action: &str, xml: Option<&str>) {
        let conn = match self.conn.conn() {
            Ok(c) => c,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(self.uri.clone(), e.to_string()));
                return;
            }
        };
        let result: Result<(), String> = match action {
            "create" => {
                Network::define_xml(conn, xml.unwrap_or(""))
                    .and_then(|n| n.create().map(|_| ()))
                    .map_err(|e| e.to_string())
            }
            "delete" => {
                Network::lookup_by_name(conn, net_name)
                    .and_then(|n| { let _ = n.destroy(); n.undefine() })
                    .map_err(|e| e.to_string())
            }
            "start" => {
                Network::lookup_by_name(conn, net_name)
                    .and_then(|n| n.create().map(|_| ()))
                    .map_err(|e| e.to_string())
            }
            "stop" => {
                Network::lookup_by_name(conn, net_name)
                    .and_then(|n| n.destroy().map(|_| ()))
                    .map_err(|e| e.to_string())
            }
            _ => Err(format!("Unknown network action: {action}")),
        };
        if let Err(e) = result {
            let _ = self.event_tx.send(BackendEvent::Error(
                self.uri.clone(),
                format!("Network '{net_name}' {action} failed: {e}"),
            ));
        }
        self.handle_list_networks();
    }

    fn handle_list_snapshots(&self, domain_name: &str) {
        let conn = match self.conn.conn() {
            Ok(c) => c,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(self.uri.clone(), e.to_string()));
                return;
            }
        };
        let domain = match Domain::lookup_by_name(conn, domain_name) {
            Ok(d) => d,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Domain '{domain_name}' not found: {e}"),
                ));
                return;
            }
        };
        let snaps = match domain.list_all_snapshots(0) {
            Ok(s) => s,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("List snapshots failed: {e}"),
                ));
                return;
            }
        };
        let current_name = DomainSnapshot::current(&domain, 0)
            .ok()
            .and_then(|s| s.get_name().ok());
        let mut summaries = Vec::new();
        for snap in &snaps {
            if let Ok(name) = snap.get_name() {
                let xml = snap.get_xml_desc(0).unwrap_or_default();
                let desc = extract_xml_text(&xml, "description").unwrap_or_default();
                let state = extract_xml_text(&xml, "state").unwrap_or_default();
                let ctime: i64 = extract_xml_text(&xml, "creationTime")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                summaries.push(SnapshotSummary {
                    is_current: current_name.as_deref() == Some(name.as_str()),
                    name,
                    description: desc,
                    state,
                    creation_time: ctime,
                });
            }
        }
        let _ = self.event_tx.send(BackendEvent::SnapshotListUpdated(
            self.uri.clone(),
            domain_name.to_string(),
            summaries,
        ));
    }

    fn handle_snapshot_action(&self, domain_name: &str, action: &str, xml_or_name: &str) {
        let conn = match self.conn.conn() {
            Ok(c) => c,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(self.uri.clone(), e.to_string()));
                return;
            }
        };
        let domain = match Domain::lookup_by_name(conn, domain_name) {
            Ok(d) => d,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Domain '{domain_name}' not found: {e}"),
                ));
                return;
            }
        };
        let result: Result<(), String> = match action {
            "create" => {
                DomainSnapshot::create_xml(&domain, xml_or_name, 0)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
            "delete" => {
                DomainSnapshot::lookup_by_name(&domain, xml_or_name, 0)
                    .and_then(|s| s.delete(0))
                    .map_err(|e| e.to_string())
            }
            "revert" => {
                DomainSnapshot::lookup_by_name(&domain, xml_or_name, 0)
                    .and_then(|s| s.revert(0))
                    .map_err(|e| e.to_string())
            }
            _ => Err(format!("Unknown snapshot action: {action}")),
        };
        if let Err(e) = result {
            let _ = self.event_tx.send(BackendEvent::Error(
                self.uri.clone(),
                format!("Snapshot '{xml_or_name}' {action} failed: {e}"),
            ));
        }
        self.handle_list_snapshots(domain_name);
    }

    fn handle_update_device(&self, domain_name: &str, device_xml: &str, flags: u32) {
        let conn = match self.conn.conn() {
            Ok(c) => c,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(self.uri.clone(), e.to_string()));
                return;
            }
        };
        let domain = match Domain::lookup_by_name(conn, domain_name) {
            Ok(d) => d,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Domain '{domain_name}' not found: {e}"),
                ));
                return;
            }
        };
        match domain.update_device_flags(device_xml, flags) {
            Ok(_) => {
                log::info!("[{}] Device updated for '{domain_name}'", self.uri);
            }
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Device update for '{domain_name}' failed: {e}"),
                ));
            }
        }
    }

    fn handle_query_qemu_caps(&self, arch: &str) {
        let caps = crate::qemu_capabilities::QemuCapabilities::query(arch);
        let _ = self.event_tx.send(BackendEvent::QemuCapsUpdated(
            self.uri.clone(),
            arch.to_string(),
            caps,
        ));
    }

    fn handle_migrate(&self, domain_name: &str, dest_uri: &str, flags: u32) {
        let conn = match self.conn.conn() {
            Ok(c) => c,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(self.uri.clone(), e.to_string()));
                return;
            }
        };
        let domain = match Domain::lookup_by_name(conn, domain_name) {
            Ok(d) => d,
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Domain '{domain_name}' not found: {e}"),
                ));
                return;
            }
        };
        match domain.migrate_to_uri(dest_uri, flags, None, 0) {
            Ok(_) => {
                let _ = self.event_tx.send(BackendEvent::MigrationComplete(
                    self.uri.clone(),
                    domain_name.to_string(),
                ));
            }
            Err(e) => {
                let _ = self.event_tx.send(BackendEvent::Error(
                    self.uri.clone(),
                    format!("Migration of '{domain_name}' failed: {e}"),
                ));
            }
        }
    }

    fn run(mut self, cmd_rx: Receiver<BackendCommand>) {
        if !self.connect() {
            return;
        }

        self.poll_domains();

        let poll_interval = Duration::from_secs(2);
        let mut last_poll = Instant::now();

        loop {
            match cmd_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(BackendCommand::Disconnect(uri)) if uri == self.uri => {
                    self.conn.close();
                    let _ = self.event_tx.send(BackendEvent::ConnectionClosed(self.uri.clone()));
                    return;
                }
                Ok(BackendCommand::DomainAction(uri, name, action)) if uri == self.uri => {
                    self.handle_domain_action(&name, action);
                    self.poll_domains();
                    last_poll = Instant::now();
                }
                Ok(BackendCommand::DefineXml(uri, xml)) if uri == self.uri => {
                    self.handle_define_xml(&xml);
                    self.poll_domains();
                    last_poll = Instant::now();
                }
                Ok(BackendCommand::UndefineDomain(uri, name)) if uri == self.uri => {
                    self.handle_undefine_domain(&name);
                    self.poll_domains();
                    last_poll = Instant::now();
                }
                Ok(BackendCommand::GetHostInfo(uri)) if uri == self.uri => {
                    self.handle_get_host_info();
                }
                Ok(BackendCommand::ListStoragePools(uri)) if uri == self.uri => {
                    self.handle_list_pools();
                }
                Ok(BackendCommand::CreateStoragePool(uri, xml)) if uri == self.uri => {
                    self.handle_storage_pool_action("", "create", Some(&xml));
                }
                Ok(BackendCommand::DeleteStoragePool(uri, name)) if uri == self.uri => {
                    self.handle_storage_pool_action(&name, "delete", None);
                }
                Ok(BackendCommand::StartStoragePool(uri, name)) if uri == self.uri => {
                    self.handle_storage_pool_action(&name, "start", None);
                }
                Ok(BackendCommand::StopStoragePool(uri, name)) if uri == self.uri => {
                    self.handle_storage_pool_action(&name, "stop", None);
                }
                Ok(BackendCommand::RefreshStoragePool(uri, name)) if uri == self.uri => {
                    self.handle_storage_pool_action(&name, "refresh", None);
                }
                Ok(BackendCommand::ListVolumes(uri, pool)) if uri == self.uri => {
                    self.handle_list_volumes(&pool);
                }
                Ok(BackendCommand::CreateVolume(uri, pool, xml)) if uri == self.uri => {
                    self.handle_volume_action(&pool, "create", &xml);
                }
                Ok(BackendCommand::DeleteVolume(uri, pool, name)) if uri == self.uri => {
                    self.handle_volume_action(&pool, "delete", &name);
                }
                Ok(BackendCommand::ListNetworks(uri)) if uri == self.uri => {
                    self.handle_list_networks();
                }
                Ok(BackendCommand::CreateNetwork(uri, xml)) if uri == self.uri => {
                    self.handle_network_action("", "create", Some(&xml));
                }
                Ok(BackendCommand::DeleteNetwork(uri, name)) if uri == self.uri => {
                    self.handle_network_action(&name, "delete", None);
                }
                Ok(BackendCommand::StartNetwork(uri, name)) if uri == self.uri => {
                    self.handle_network_action(&name, "start", None);
                }
                Ok(BackendCommand::StopNetwork(uri, name)) if uri == self.uri => {
                    self.handle_network_action(&name, "stop", None);
                }
                Ok(BackendCommand::ListSnapshots(uri, domain)) if uri == self.uri => {
                    self.handle_list_snapshots(&domain);
                }
                Ok(BackendCommand::CreateSnapshot(uri, domain, xml)) if uri == self.uri => {
                    self.handle_snapshot_action(&domain, "create", &xml);
                }
                Ok(BackendCommand::DeleteSnapshot(uri, domain, name)) if uri == self.uri => {
                    self.handle_snapshot_action(&domain, "delete", &name);
                }
                Ok(BackendCommand::RevertSnapshot(uri, domain, name)) if uri == self.uri => {
                    self.handle_snapshot_action(&domain, "revert", &name);
                }
                Ok(BackendCommand::MigrateDomain(uri, name, dest, flags)) if uri == self.uri => {
                    self.handle_migrate(&name, &dest, flags);
                }
                Ok(BackendCommand::UpdateDevice(uri, name, xml, flags)) if uri == self.uri => {
                    self.handle_update_device(&name, &xml, flags);
                    self.poll_domains();
                    last_poll = Instant::now();
                }
                Ok(BackendCommand::QueryQemuCaps(uri, arch)) if uri == self.uri => {
                    self.handle_query_qemu_caps(&arch);
                }
                Ok(BackendCommand::PollAll(uri)) if uri == self.uri => {
                    self.poll_domains();
                    last_poll = Instant::now();
                }
                Ok(_) => {}
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return,
            }

            if last_poll.elapsed() >= poll_interval {
                self.poll_domains();
                last_poll = Instant::now();
            }
        }
    }
}

pub struct BackendManager {
    event_rx: Receiver<BackendEvent>,
    event_tx: Sender<BackendEvent>,
    workers: HashMap<String, Sender<BackendCommand>>,
}

impl BackendManager {
    pub fn new() -> Self {
        let (event_tx, event_rx) = crossbeam_channel::unbounded();
        Self {
            event_rx,
            event_tx,
            workers: HashMap::new(),
        }
    }

    pub fn start_connection(&mut self, uri: String) {
        if self.workers.contains_key(&uri) {
            return;
        }

        let event_tx = self.event_tx.clone();
        let (worker_tx, worker_rx) = crossbeam_channel::unbounded();

        let uri_clone = uri.clone();
        thread::Builder::new()
            .name(format!("libvirt-{uri}"))
            .spawn(move || {
                let worker = ConnectionWorker::new(uri_clone, event_tx);
                worker.run(worker_rx);
            })
            .expect("Failed to spawn worker thread");

        self.workers.insert(uri, worker_tx);
    }

    pub fn stop_connection(&mut self, uri: &str) {
        if let Some(tx) = self.workers.get(uri) {
            let _ = tx.send(BackendCommand::Disconnect(uri.to_string()));
        }
        self.workers.remove(uri);
    }

    pub fn send_to(&self, uri: &str, cmd: BackendCommand) {
        if let Some(tx) = self.workers.get(uri) {
            let _ = tx.send(cmd);
        }
    }

    pub fn drain_events(&self) -> Vec<BackendEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }
}

fn extract_xml_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}
