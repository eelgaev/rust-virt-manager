use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRng {
    #[serde(rename = "@model")]
    pub model: String,

    #[serde(default)]
    pub backend: Option<RngBackend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RngBackend {
    #[serde(rename = "@model")]
    pub model: String,

    #[serde(rename = "$text", default)]
    pub device: Option<String>,
}

impl DeviceRng {
    pub fn display_name(&self) -> String {
        if let Some(backend) = &self.backend {
            if let Some(dev) = &backend.device {
                return format!("RNG {dev}");
            }
        }
        "RNG".to_string()
    }
}
