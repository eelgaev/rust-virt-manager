use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceHostdev {
    #[serde(rename = "@mode")]
    pub mode: String,

    #[serde(rename = "@type")]
    pub hostdev_type: String,

    #[serde(rename = "@managed", default)]
    pub managed: Option<String>,

    #[serde(default)]
    pub source: Option<HostdevSource>,

    #[serde(default)]
    pub rom: Option<HostdevRom>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostdevSource {
    #[serde(default)]
    pub vendor: Option<HostdevId>,

    #[serde(default)]
    pub product: Option<HostdevId>,

    #[serde(default)]
    pub address: Option<HostdevAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostdevId {
    #[serde(rename = "@id")]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostdevAddress {
    #[serde(rename = "@domain", default)]
    pub domain: Option<String>,

    #[serde(rename = "@bus", default)]
    pub bus: Option<String>,

    #[serde(rename = "@slot", default)]
    pub slot: Option<String>,

    #[serde(rename = "@function", default)]
    pub function: Option<String>,

    #[serde(rename = "@device", default)]
    pub device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostdevRom {
    #[serde(rename = "@bar", default)]
    pub bar: Option<String>,
}

impl DeviceHostdev {
    pub fn display_name(&self) -> String {
        let type_label = match self.hostdev_type.as_str() {
            "usb" => "USB",
            "pci" => "PCI",
            "scsi" => "SCSI",
            "mdev" => "MDEV",
            _ => &self.hostdev_type,
        };

        if let Some(source) = &self.source {
            if let (Some(vendor), Some(product)) = (&source.vendor, &source.product) {
                return format!("{type_label} {}:{}", vendor.id, product.id);
            }
        }
        format!("{type_label} Host Device")
    }
}
