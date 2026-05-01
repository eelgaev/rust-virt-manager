use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePanic {
    #[serde(rename = "@model")]
    pub model: String,
}

impl DevicePanic {
    pub fn display_name(&self) -> String {
        format!("Panic {}", self.model)
    }
}
