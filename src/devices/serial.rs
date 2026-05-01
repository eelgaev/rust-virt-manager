use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSerial {
    #[serde(rename = "@type")]
    pub serial_type: String,

    #[serde(default)]
    pub target: Option<SerialTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialTarget {
    #[serde(rename = "@type", default)]
    pub target_type: Option<String>,

    #[serde(rename = "@port", default)]
    pub port: Option<String>,

    #[serde(default)]
    pub model: Option<SerialTargetModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialTargetModel {
    #[serde(rename = "@name", default)]
    pub name: Option<String>,
}

impl DeviceSerial {
    pub fn display_name(&self, index: usize) -> String {
        format!("Serial {}", index + 1)
    }
}
