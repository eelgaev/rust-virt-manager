use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSound {
    #[serde(rename = "@model")]
    pub model: String,
}

impl DeviceSound {
    pub fn display_name(&self) -> String {
        format!("Sound {}", self.model)
    }
}
