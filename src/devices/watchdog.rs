use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceWatchdog {
    #[serde(rename = "@model")]
    pub model: String,

    #[serde(rename = "@action", default)]
    pub action: Option<String>,
}

impl DeviceWatchdog {
    pub fn display_name(&self) -> String {
        format!("Watchdog {}", self.model)
    }
}
