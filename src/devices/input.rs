use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInput {
    #[serde(rename = "@type")]
    pub input_type: String,

    #[serde(rename = "@bus", default)]
    pub bus: Option<String>,
}

impl DeviceInput {
    pub fn display_name(&self) -> String {
        let label = match self.input_type.as_str() {
            "tablet" => "Tablet",
            "mouse" => "Mouse",
            "keyboard" => "Keyboard",
            "evdev" => "EvDev",
            _ => &self.input_type,
        };
        label.to_string()
    }
}
