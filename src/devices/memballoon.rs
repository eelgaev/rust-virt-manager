use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceMemballoon {
    #[serde(rename = "@model")]
    pub model: String,

    #[serde(rename = "@autodeflate", default)]
    pub autodeflate: Option<String>,
}

impl DeviceMemballoon {
    pub fn display_name(&self) -> String {
        format!("Memballoon {}", self.model)
    }
}
