use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceGraphics {
    #[serde(rename = "@type")]
    pub graphics_type: String,

    #[serde(rename = "@port", default)]
    pub port: Option<i32>,

    #[serde(rename = "@autoport", default)]
    pub autoport: Option<String>,

    #[serde(rename = "@listen", default)]
    pub listen: Option<String>,

    #[serde(rename = "@passwd", default)]
    pub passwd: Option<String>,

    #[serde(rename = "@keymap", default)]
    pub keymap: Option<String>,
}

impl DeviceGraphics {
    pub fn is_vnc(&self) -> bool {
        self.graphics_type == "vnc"
    }

    pub fn is_spice(&self) -> bool {
        self.graphics_type == "spice"
    }

    pub fn display_name(&self) -> String {
        let port_str = self.port
            .filter(|&p| p > 0)
            .map(|p| format!(" :{}", p))
            .unwrap_or_default();
        format!("{}{}", self.graphics_type.to_uppercase(), port_str)
    }
}
