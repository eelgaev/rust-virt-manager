use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRedirdev {
    #[serde(rename = "@bus")]
    pub bus: String,

    #[serde(rename = "@type")]
    pub redir_type: String,
}

impl DeviceRedirdev {
    pub fn display_name(&self, index: usize) -> String {
        format!("{} Redirector {}", self.bus.to_uppercase(), index + 1)
    }
}
