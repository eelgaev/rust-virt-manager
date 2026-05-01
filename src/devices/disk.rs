use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDisk {
    #[serde(rename = "@type")]
    pub disk_type: String,

    #[serde(rename = "@device", default = "default_disk_device")]
    pub device: String,

    #[serde(default)]
    pub driver: Option<DiskDriver>,

    #[serde(default)]
    pub source: Option<DiskSource>,

    pub target: DiskTarget,

    #[serde(default)]
    pub readonly: Option<Empty>,

    #[serde(default)]
    pub address: Option<DiskAddress>,
}

fn default_disk_device() -> String {
    "disk".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskDriver {
    #[serde(rename = "@name", default)]
    pub name: Option<String>,

    #[serde(rename = "@type", default)]
    pub driver_type: Option<String>,

    #[serde(rename = "@cache", default)]
    pub cache: Option<String>,

    #[serde(rename = "@discard", default)]
    pub discard: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskSource {
    #[serde(rename = "@file", default)]
    pub file: Option<String>,

    #[serde(rename = "@dev", default)]
    pub dev: Option<String>,

    #[serde(rename = "@pool", default)]
    pub pool: Option<String>,

    #[serde(rename = "@volume", default)]
    pub volume: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskTarget {
    #[serde(rename = "@dev")]
    pub dev: String,

    #[serde(rename = "@bus", default)]
    pub bus: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskAddress {
    #[serde(rename = "@type")]
    pub address_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Empty {}

impl DeviceDisk {
    pub fn source_path(&self) -> Option<&str> {
        self.source.as_ref().and_then(|s| {
            s.file.as_deref().or(s.dev.as_deref())
        })
    }

    pub fn is_cdrom(&self) -> bool {
        self.device == "cdrom"
    }

    pub fn is_floppy(&self) -> bool {
        self.device == "floppy"
    }

    pub fn display_name(&self) -> String {
        let bus = self.target.bus.as_deref().unwrap_or("unknown");
        format!("{} {} ({})", self.device, self.target.dev, bus)
    }
}
