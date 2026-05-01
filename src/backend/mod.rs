pub mod worker;

use std::collections::VecDeque;

use crate::domain::DomainState;

#[derive(Debug, Clone)]
pub enum BackendCommand {
    Connect(String),
    Disconnect(String),
    DomainAction(String, String, DomainAction),
    DefineXml(String, String),
    UndefineDomain(String, String),
    PollAll(String),
    GetHostInfo(String),
    ListStoragePools(String),
    CreateStoragePool(String, String),
    DeleteStoragePool(String, String),
    StartStoragePool(String, String),
    StopStoragePool(String, String),
    RefreshStoragePool(String, String),
    ListVolumes(String, String),
    CreateVolume(String, String, String),
    DeleteVolume(String, String, String),
    ListNetworks(String),
    CreateNetwork(String, String),
    DeleteNetwork(String, String),
    StartNetwork(String, String),
    StopNetwork(String, String),
    ListSnapshots(String, String),
    CreateSnapshot(String, String, String),
    DeleteSnapshot(String, String, String),
    RevertSnapshot(String, String, String),
    MigrateDomain(String, String, String, u32),
    UpdateDevice(String, String, String, u32),
    QueryQemuCaps(String, String),
}

#[derive(Debug, Clone)]
pub enum DomainAction {
    Start,
    Shutdown,
    ForceOff,
    Pause,
    Resume,
    Reboot,
}

#[derive(Debug, Clone)]
pub enum BackendEvent {
    ConnectionOpened(String),
    ConnectionFailed(String, String),
    ConnectionClosed(String),
    DomainListUpdated(String, Vec<DomainSummary>),
    DomainStateChanged {
        uri: String,
        name: String,
        state: DomainState,
    },
    DomainStatsUpdated(String, Vec<DomainStats>),
    DomainDefined(String, String),
    DomainUndefined(String, String),
    HostInfoUpdated(String, HostInfo),
    StoragePoolListUpdated(String, Vec<StoragePoolSummary>),
    VolumeListUpdated(String, String, Vec<VolumeSummary>),
    NetworkListUpdated(String, Vec<NetworkSummary>),
    SnapshotListUpdated(String, String, Vec<SnapshotSummary>),
    MigrationComplete(String, String),
    QemuCapsUpdated(String, String, crate::qemu_capabilities::QemuCapabilities),
    Error(String, String),
}

#[derive(Debug, Clone)]
pub struct HostInfo {
    pub hostname: String,
    pub arch: String,
    pub memory_kb: u64,
    pub cpus: u32,
    pub mhz: u32,
    pub model: String,
    pub lib_version: u32,
}

#[derive(Debug, Clone)]
pub struct StoragePoolSummary {
    pub name: String,
    pub uuid: String,
    pub active: bool,
    pub autostart: bool,
    pub capacity: u64,
    pub allocation: u64,
    pub available: u64,
    pub xml: String,
}

#[derive(Debug, Clone)]
pub struct VolumeSummary {
    pub name: String,
    pub path: String,
    pub capacity: u64,
    pub allocation: u64,
    pub vol_type: String,
}

#[derive(Debug, Clone)]
pub struct NetworkSummary {
    pub name: String,
    pub uuid: String,
    pub active: bool,
    pub autostart: bool,
    pub bridge: String,
    pub xml: String,
}

#[derive(Debug, Clone)]
pub struct SnapshotSummary {
    pub name: String,
    pub description: String,
    pub state: String,
    pub creation_time: i64,
    pub is_current: bool,
}

#[derive(Debug, Clone)]
pub struct DomainSummary {
    pub name: String,
    pub uuid: String,
    pub state: DomainState,
    pub vcpus: u32,
    pub memory_kib: u64,
    pub xml: String,
}

#[derive(Debug, Clone, Default)]
pub struct DomainStats {
    pub name: String,
    pub cpu_time_ns: u64,
    pub cpu_percent: f64,
    pub memory_rss_kib: u64,
    pub disk_rd_bytes: u64,
    pub disk_wr_bytes: u64,
    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub struct StatsHistory {
    pub cpu: VecDeque<f64>,
    pub memory: VecDeque<f64>,
    pub disk_rd: VecDeque<f64>,
    pub disk_wr: VecDeque<f64>,
    pub net_rx: VecDeque<f64>,
    pub net_tx: VecDeque<f64>,
}

impl StatsHistory {
    pub const MAX_POINTS: usize = 40;

    pub fn push_cpu(&mut self, val: f64) {
        self.cpu.push_back(val);
        if self.cpu.len() > Self::MAX_POINTS {
            self.cpu.pop_front();
        }
    }

    pub fn push_memory(&mut self, val: f64) {
        self.memory.push_back(val);
        if self.memory.len() > Self::MAX_POINTS {
            self.memory.pop_front();
        }
    }

    pub fn push_disk(&mut self, rd: f64, wr: f64) {
        self.disk_rd.push_back(rd);
        self.disk_wr.push_back(wr);
        if self.disk_rd.len() > Self::MAX_POINTS {
            self.disk_rd.pop_front();
        }
        if self.disk_wr.len() > Self::MAX_POINTS {
            self.disk_wr.pop_front();
        }
    }

    pub fn push_net(&mut self, rx: f64, tx: f64) {
        self.net_rx.push_back(rx);
        self.net_tx.push_back(tx);
        if self.net_rx.len() > Self::MAX_POINTS {
            self.net_rx.pop_front();
        }
        if self.net_tx.len() > Self::MAX_POINTS {
            self.net_tx.pop_front();
        }
    }
}
