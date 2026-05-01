use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSmartcard {
    #[serde(rename = "@mode")]
    pub mode: String,

    #[serde(rename = "@type", default)]
    pub smartcard_type: Option<String>,
}

impl DeviceSmartcard {
    pub fn display_name(&self) -> String {
        format!("Smartcard {}", self.mode)
    }
}
