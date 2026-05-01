use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceChannel {
    #[serde(rename = "@type")]
    pub channel_type: String,

    #[serde(default)]
    pub target: Option<ChannelTarget>,

    #[serde(default)]
    pub source: Option<ChannelSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelTarget {
    #[serde(rename = "@type", default)]
    pub target_type: Option<String>,

    #[serde(rename = "@name", default)]
    pub name: Option<String>,

    #[serde(rename = "@state", default)]
    pub state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSource {
    #[serde(rename = "@mode", default)]
    pub mode: Option<String>,

    #[serde(rename = "@path", default)]
    pub path: Option<String>,
}

impl DeviceChannel {
    pub fn display_name(&self) -> String {
        if let Some(target) = &self.target {
            if let Some(name) = &target.name {
                return format!("Channel {name}");
            }
            if let Some(tt) = &target.target_type {
                return format!("Channel {tt}");
            }
        }
        format!("Channel {}", self.channel_type)
    }
}
