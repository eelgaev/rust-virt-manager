use serde::{Deserialize, Serialize};

use crate::devices::{
    channel::DeviceChannel, console::DeviceConsole, controller::DeviceController,
    disk::DeviceDisk, filesystem::DeviceFilesystem, graphics::DeviceGraphics,
    hostdev::DeviceHostdev, input::DeviceInput, interface::DeviceInterface,
    memballoon::DeviceMemballoon, panic_dev::DevicePanic, redirdev::DeviceRedirdev,
    rng::DeviceRng, serial::DeviceSerial, smartcard::DeviceSmartcard, sound::DeviceSound,
    tpm::DeviceTpm, video::DeviceVideo, vsock::DeviceVsock, watchdog::DeviceWatchdog,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "domain")]
pub struct Guest {
    #[serde(rename = "@type")]
    pub domain_type: String,

    pub name: String,

    #[serde(default)]
    pub uuid: Option<String>,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default)]
    pub description: Option<String>,

    pub memory: MemoryConfig,

    #[serde(default, rename = "currentMemory")]
    pub current_memory: Option<MemoryConfig>,

    pub vcpu: VcpuConfig,

    pub os: OsConfig,

    #[serde(default)]
    pub devices: Option<DevicesSection>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DevicesSection {
    #[serde(default, rename = "disk")]
    pub disks: Vec<DeviceDisk>,

    #[serde(default, rename = "interface")]
    pub interfaces: Vec<DeviceInterface>,

    #[serde(default, rename = "graphics")]
    pub graphics: Vec<DeviceGraphics>,

    #[serde(default, rename = "video")]
    pub videos: Vec<DeviceVideo>,

    #[serde(default, rename = "sound")]
    pub sounds: Vec<DeviceSound>,

    #[serde(default, rename = "input")]
    pub inputs: Vec<DeviceInput>,

    #[serde(default, rename = "controller")]
    pub controllers: Vec<DeviceController>,

    #[serde(default, rename = "channel")]
    pub channels: Vec<DeviceChannel>,

    #[serde(default, rename = "console")]
    pub consoles: Vec<DeviceConsole>,

    #[serde(default, rename = "serial")]
    pub serials: Vec<DeviceSerial>,

    #[serde(default, rename = "watchdog")]
    pub watchdogs: Vec<DeviceWatchdog>,

    #[serde(default, rename = "rng")]
    pub rngs: Vec<DeviceRng>,

    #[serde(default, rename = "tpm")]
    pub tpms: Vec<DeviceTpm>,

    #[serde(default, rename = "hostdev")]
    pub hostdevs: Vec<DeviceHostdev>,

    #[serde(default, rename = "filesystem")]
    pub filesystems: Vec<DeviceFilesystem>,

    #[serde(default, rename = "memballoon")]
    pub memballoons: Vec<DeviceMemballoon>,

    #[serde(default, rename = "vsock")]
    pub vsocks: Vec<DeviceVsock>,

    #[serde(default, rename = "redirdev")]
    pub redirdevs: Vec<DeviceRedirdev>,

    #[serde(default, rename = "smartcard")]
    pub smartcards: Vec<DeviceSmartcard>,

    #[serde(default, rename = "panic")]
    pub panics: Vec<DevicePanic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(rename = "@unit", default = "default_memory_unit")]
    pub unit: String,

    #[serde(rename = "$text")]
    pub value: u64,
}

fn default_memory_unit() -> String {
    "KiB".to_string()
}

impl MemoryConfig {
    pub fn as_kib(&self) -> u64 {
        match self.unit.as_str() {
            "b" | "bytes" => self.value / 1024,
            "KB" => self.value * 1000 / 1024,
            "KiB" | "K" | "k" => self.value,
            "MB" => self.value * 1000 * 1000 / 1024,
            "MiB" | "M" => self.value * 1024,
            "GB" => self.value * 1000 * 1000 * 1000 / 1024,
            "GiB" | "G" => self.value * 1024 * 1024,
            "TB" => self.value * 1000 * 1000 * 1000 * 1000 / 1024,
            "TiB" | "T" => self.value * 1024 * 1024 * 1024,
            _ => self.value,
        }
    }

    pub fn as_mib(&self) -> u64 {
        self.as_kib() / 1024
    }

    pub fn as_gib_f64(&self) -> f64 {
        self.as_kib() as f64 / (1024.0 * 1024.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcpuConfig {
    #[serde(rename = "$text")]
    pub count: u32,

    #[serde(rename = "@placement", default)]
    pub placement: Option<String>,

    #[serde(rename = "@current", default)]
    pub current: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsConfig {
    #[serde(rename = "type")]
    pub os_type: OsType,

    #[serde(default)]
    pub boot: Vec<BootDevice>,

    #[serde(default)]
    pub loader: Option<OsLoader>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsType {
    #[serde(rename = "$text")]
    pub value: String,

    #[serde(rename = "@arch", default)]
    pub arch: Option<String>,

    #[serde(rename = "@machine", default)]
    pub machine: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootDevice {
    #[serde(rename = "@dev")]
    pub dev: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsLoader {
    #[serde(rename = "$text", default)]
    pub path: Option<String>,

    #[serde(rename = "@readonly", default)]
    pub readonly: Option<String>,

    #[serde(rename = "@type", default)]
    pub loader_type: Option<String>,

    #[serde(rename = "@secure", default)]
    pub secure: Option<String>,
}

impl Guest {
    pub fn from_xml(xml: &str) -> crate::error::Result<Self> {
        Ok(quick_xml::de::from_str(xml)?)
    }

    pub fn effective_vcpus(&self) -> u32 {
        self.vcpu.current.unwrap_or(self.vcpu.count)
    }

    pub fn memory_mib(&self) -> u64 {
        self.memory.as_mib()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainState {
    NoState,
    Running,
    Blocked,
    Paused,
    Shutdown,
    Shutoff,
    Crashed,
    PmSuspended,
    Unknown,
}

impl DomainState {
    pub fn from_libvirt(state: u32) -> Self {
        match state {
            0 => Self::NoState,
            1 => Self::Running,
            2 => Self::Blocked,
            3 => Self::Paused,
            4 => Self::Shutdown,
            5 => Self::Shutoff,
            6 => Self::Crashed,
            7 => Self::PmSuspended,
            _ => Self::Unknown,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::NoState => "No state",
            Self::Running => "Running",
            Self::Blocked => "Blocked",
            Self::Paused => "Paused",
            Self::Shutdown => "Shutting down",
            Self::Shutoff => "Shut off",
            Self::Crashed => "Crashed",
            Self::PmSuspended => "Suspended",
            Self::Unknown => "Unknown",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running | Self::Blocked | Self::Paused)
    }
}
