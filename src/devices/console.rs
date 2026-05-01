use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConsole {
    #[serde(rename = "@type")]
    pub console_type: String,

    #[serde(default)]
    pub target: Option<ConsoleTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleTarget {
    #[serde(rename = "@type", default)]
    pub target_type: Option<String>,

    #[serde(rename = "@port", default)]
    pub port: Option<String>,
}

impl DeviceConsole {
    pub fn display_name(&self, index: usize) -> String {
        format!("Console {}", index + 1)
    }
}
