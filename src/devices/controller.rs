use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceController {
    #[serde(rename = "@type")]
    pub controller_type: String,

    #[serde(rename = "@model", default)]
    pub model: Option<String>,

    #[serde(rename = "@index", default)]
    pub index: Option<u32>,

    #[serde(rename = "@ports", default)]
    pub ports: Option<u32>,
}

impl DeviceController {
    pub fn display_name(&self) -> String {
        let idx = self.index.unwrap_or(0);
        format!("Controller {} {}", self.controller_type.to_uppercase(), idx)
    }
}
