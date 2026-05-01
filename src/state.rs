use std::collections::HashMap;

use crate::domain::DomainState;
use crate::backend::{DomainSummary, StatsHistory};
use crate::backend::worker::BackendManager;
use crate::config::AppConfig;
use crate::edit_state::DeviceEdits;
use crate::qemu_capabilities::QemuCapabilities;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Active,
}

pub struct GuiConnection {
    pub uri: String,
    pub state: ConnectionState,
    pub domains: HashMap<String, GuiDomain>,
    pub expanded: bool,
}

impl GuiConnection {
    pub fn new(uri: String) -> Self {
        Self {
            uri,
            state: ConnectionState::Disconnected,
            domains: HashMap::new(),
            expanded: true,
        }
    }
}

pub struct GuiDomain {
    pub name: String,
    pub uuid: String,
    pub state: DomainState,
    pub vcpus: u32,
    pub memory_kib: u64,
    pub xml: String,
    pub stats: StatsHistory,
}

impl GuiDomain {
    pub fn from_summary(summary: &DomainSummary) -> Self {
        Self {
            name: summary.name.clone(),
            uuid: summary.uuid.clone(),
            state: summary.state,
            vcpus: summary.vcpus,
            memory_kib: summary.memory_kib,
            xml: summary.xml.clone(),
            stats: StatsHistory::default(),
        }
    }

    pub fn memory_display(&self) -> String {
        let mib = self.memory_kib / 1024;
        if mib >= 1024 {
            format!("{:.1} GiB", mib as f64 / 1024.0)
        } else {
            format!("{mib} MiB")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hypervisor {
    QemuKvm,
    QemuSession,
    Xen,
    CustomUri,
}

impl Hypervisor {
    pub const ALL: &[Hypervisor] = &[
        Hypervisor::QemuKvm,
        Hypervisor::QemuSession,
        Hypervisor::Xen,
        Hypervisor::CustomUri,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::QemuKvm => "QEMU/KVM",
            Self::QemuSession => "QEMU/KVM user session",
            Self::Xen => "Xen",
            Self::CustomUri => "Custom URI...",
        }
    }

    fn uri_scheme(&self) -> &'static str {
        match self {
            Self::QemuKvm | Self::QemuSession => "qemu",
            Self::Xen => "xen",
            Self::CustomUri => "",
        }
    }

    fn uri_path(&self) -> &'static str {
        match self {
            Self::QemuKvm | Self::Xen => "/system",
            Self::QemuSession => "/session",
            Self::CustomUri => "",
        }
    }

    pub fn supports_remote(&self) -> bool {
        matches!(self, Self::QemuKvm | Self::Xen)
    }
}

pub struct AddConnectionState {
    pub open: bool,
    pub hypervisor: Hypervisor,
    pub connect_remote: bool,
    pub username: String,
    pub hostname: String,
    pub autoconnect: bool,
    pub custom_uri: String,
}

impl Default for AddConnectionState {
    fn default() -> Self {
        Self {
            open: false,
            hypervisor: Hypervisor::QemuKvm,
            connect_remote: false,
            username: String::new(),
            hostname: String::new(),
            autoconnect: true,
            custom_uri: String::new(),
        }
    }
}

impl AddConnectionState {
    pub fn generated_uri(&self) -> String {
        if self.hypervisor == Hypervisor::CustomUri {
            return self.custom_uri.clone();
        }

        let scheme = self.hypervisor.uri_scheme();
        let path = self.hypervisor.uri_path();

        if self.connect_remote && self.hypervisor.supports_remote() {
            let user_part = if self.username.is_empty() {
                String::new()
            } else {
                format!("{}@", self.username)
            };
            format!("{scheme}+ssh://{user_part}{}{path}", self.hostname)
        } else {
            format!("{scheme}:///{}", &path[1..])
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmTab {
    Details,
    Console,
    Snapshots,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HwListItem {
    Overview,
    Performance,
    Cpu,
    Memory,
    Boot,
    Disk(usize),
    Nic(usize),
    Graphics(usize),
    Video(usize),
    Sound(usize),
    Input(usize),
    Char(usize),
    Controller(usize),
    Hostdev(usize),
    Watchdog(usize),
    Filesystem(usize),
    Tpm(usize),
    Rng(usize),
    Vsock(usize),
    Redirdev(usize),
    Smartcard(usize),
    Panic(usize),
}

pub struct VmWindowState {
    pub uri: String,
    pub domain_name: String,
    pub active_tab: VmTab,
    pub selected_hw: HwListItem,
    pub show_xml_editor: bool,
    pub xml_editor_text: String,
    pub device_edits: DeviceEdits,
    pub snapshots: Vec<crate::backend::SnapshotSummary>,
    pub selected_snapshot: Option<String>,
    pub create_snapshot_name: String,
    pub create_snapshot_desc: String,
    pub show_create_snapshot: bool,
    pub vnc_handle: Option<crate::console::vnc::VncHandle>,
    pub vnc_texture: Option<egui::TextureHandle>,
    pub serial_handle: Option<crate::console::serial::SerialHandle>,
    pub vnc_retry_after: Option<std::time::Instant>,
    pub vnc_retries: u32,
    pub ssh_tunnel: Option<crate::console::ssh_tunnel::SshTunnel>,
    pub ssh_tunnel_port: Option<u16>,
}

impl VmWindowState {
    pub fn new(uri: String, domain_name: String) -> Self {
        Self {
            uri,
            domain_name,
            active_tab: VmTab::Console,
            selected_hw: HwListItem::Overview,
            show_xml_editor: false,
            xml_editor_text: String::new(),
            device_edits: DeviceEdits::default(),
            snapshots: Vec::new(),
            selected_snapshot: None,
            create_snapshot_name: String::new(),
            create_snapshot_desc: String::new(),
            show_create_snapshot: false,
            vnc_handle: None,
            vnc_texture: None,
            serial_handle: None,
            vnc_retry_after: None,
            vnc_retries: 0,
            ssh_tunnel: None,
            ssh_tunnel_port: None,
        }
    }
}

pub struct ConnDetailsState {
    pub host_info: Option<crate::backend::HostInfo>,
    pub pools: Vec<crate::backend::StoragePoolSummary>,
    pub volumes: HashMap<String, Vec<crate::backend::VolumeSummary>>,
    pub networks: Vec<crate::backend::NetworkSummary>,
    pub active_tab: usize,
    pub selected_pool: Option<String>,
    pub selected_network: Option<String>,
    pub confirm_delete_pool: Option<String>,
    pub confirm_delete_vol: Option<String>,
}

impl ConnDetailsState {
    pub fn new() -> Self {
        Self {
            host_info: None,
            pools: Vec::new(),
            volumes: HashMap::new(),
            networks: Vec::new(),
            active_tab: 0,
            selected_pool: None,
            selected_network: None,
            confirm_delete_pool: None,
            confirm_delete_vol: None,
        }
    }
}

pub struct CreateVmState {
    pub open: bool,
    pub page: usize,
    pub uri: String,
    pub name: String,
    pub os_preset: crate::views::create_vm::OsPreset,
    pub install_method: usize,
    pub iso_path: String,
    pub import_path: String,
    pub vcpus: u32,
    pub memory_mib: u64,
    pub create_disk: bool,
    pub disk_size_gib: u64,
    pub target_pool: String,
    pub customize_before_install: bool,
}

impl Default for CreateVmState {
    fn default() -> Self {
        Self {
            open: false,
            page: 0,
            uri: "qemu:///system".into(),
            name: String::new(),
            os_preset: crate::views::create_vm::OsPreset::GenericLinux,
            install_method: 0,
            iso_path: String::new(),
            import_path: String::new(),
            vcpus: 2,
            memory_mib: 2048,
            create_disk: true,
            disk_size_gib: 20,
            target_pool: "default".into(),
            customize_before_install: false,
        }
    }
}

pub struct DeleteVmState {
    pub open: bool,
    pub uri: String,
    pub domain_name: String,
    pub delete_storage: Vec<(String, bool)>,
}

impl Default for DeleteVmState {
    fn default() -> Self {
        Self {
            open: false,
            uri: String::new(),
            domain_name: String::new(),
            delete_storage: Vec::new(),
        }
    }
}

pub struct CloneVmState {
    pub open: bool,
    pub uri: String,
    pub source_name: String,
    pub clone_name: String,
    pub disk_strategies: Vec<crate::views::clone_vm::CloneDiskStrategy>,
}

impl Default for CloneVmState {
    fn default() -> Self {
        Self {
            open: false,
            uri: String::new(),
            source_name: String::new(),
            clone_name: String::new(),
            disk_strategies: Vec::new(),
        }
    }
}

pub struct MigrateState {
    pub open: bool,
    pub uri: String,
    pub domain_name: String,
    pub dest_uri: String,
    pub live: bool,
}

impl Default for MigrateState {
    fn default() -> Self {
        Self {
            open: false,
            uri: String::new(),
            domain_name: String::new(),
            dest_uri: String::new(),
            live: true,
        }
    }
}

pub struct CreateVolumeState {
    pub open: bool,
    pub uri: String,
    pub pool_name: String,
    pub name: String,
    pub format: String,
    pub capacity_gib: u64,
}

impl Default for CreateVolumeState {
    fn default() -> Self {
        Self {
            open: false,
            uri: String::new(),
            pool_name: String::new(),
            name: String::new(),
            format: "qcow2".into(),
            capacity_gib: 10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseTarget {
    DiskSource,
    IsoSource,
    AddHwDisk,
}

pub struct VolumeBrowserState {
    pub open: bool,
    pub uri: String,
    pub target: BrowseTarget,
    pub selected_pool: Option<String>,
    pub selected_path: Option<String>,
}

impl Default for VolumeBrowserState {
    fn default() -> Self {
        Self {
            open: false,
            uri: String::new(),
            target: BrowseTarget::DiskSource,
            selected_pool: None,
            selected_path: None,
        }
    }
}

pub struct AppState {
    pub config: AppConfig,
    pub backend: BackendManager,
    pub connections: Vec<GuiConnection>,
    pub selected_uri: Option<String>,
    pub selected_domain: Option<String>,
    pub add_connection: AddConnectionState,
    pub error_message: Option<(String, std::time::Instant)>,
    pub vm_windows: HashMap<String, VmWindowState>,
    pub conn_details: HashMap<String, ConnDetailsState>,
    pub create_vm: CreateVmState,
    pub delete_vm: DeleteVmState,
    pub clone_vm: CloneVmState,
    pub migrate: MigrateState,
    pub add_hardware: crate::views::add_hardware::AddHardwareState,
    pub create_pool: crate::views::create_pool::CreatePoolState,
    pub create_network: crate::views::create_network::CreateNetworkState,
    pub create_volume: CreateVolumeState,
    pub volume_browser: VolumeBrowserState,
    pub qemu_caps: HashMap<String, QemuCapabilities>,
    pub show_preferences: bool,
    pub show_about: bool,
}

impl AppState {
    pub fn new() -> Self {
        let config = AppConfig::load();
        let mut state = Self {
            backend: BackendManager::new(),
            connections: Vec::new(),
            selected_uri: None,
            selected_domain: None,
            add_connection: AddConnectionState::default(),
            error_message: None,
            vm_windows: HashMap::new(),
            conn_details: HashMap::new(),
            create_vm: CreateVmState::default(),
            delete_vm: DeleteVmState::default(),
            clone_vm: CloneVmState::default(),
            migrate: MigrateState::default(),
            add_hardware: Default::default(),
            create_pool: Default::default(),
            create_network: Default::default(),
            create_volume: CreateVolumeState::default(),
            volume_browser: VolumeBrowserState::default(),
            qemu_caps: HashMap::new(),
            show_preferences: false,
            show_about: false,
            config,
        };

        let uris: Vec<String> = state.config.saved_uris.clone();
        for uri in &uris {
            state.connections.push(GuiConnection::new(uri.clone()));
            if state.config.auto_connect {
                state.connections.last_mut().unwrap().state = ConnectionState::Connecting;
                state.backend.start_connection(uri.clone());
            }
        }

        state
    }

    pub fn find_connection_mut(&mut self, uri: &str) -> Option<&mut GuiConnection> {
        self.connections.iter_mut().find(|c| c.uri == uri)
    }

    pub fn selected_domain_info(&self) -> Option<(&GuiConnection, &GuiDomain)> {
        let uri = self.selected_uri.as_ref()?;
        let name = self.selected_domain.as_ref()?;
        let conn = self.connections.iter().find(|c| c.uri == *uri)?;
        let domain = conn.domains.get(name)?;
        Some((conn, domain))
    }

    pub fn set_error(&mut self, msg: String) {
        self.error_message = Some((msg, std::time::Instant::now()));
    }

    pub fn save_config(&self) {
        self.config.save();
    }
}
