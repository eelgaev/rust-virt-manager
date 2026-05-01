use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceVideo {
    pub model: VideoModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoModel {
    #[serde(rename = "@type", default)]
    pub model_type: Option<String>,

    #[serde(rename = "@vram", default)]
    pub vram: Option<u64>,

    #[serde(rename = "@heads", default)]
    pub heads: Option<u32>,

    #[serde(rename = "@primary", default)]
    pub primary: Option<String>,

    #[serde(default)]
    pub acceleration: Option<VideoAcceleration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoAcceleration {
    #[serde(rename = "@accel3d", default)]
    pub accel3d: Option<String>,
}

impl DeviceVideo {
    pub fn model_type(&self) -> &str {
        self.model.model_type.as_deref().unwrap_or("unknown")
    }

    pub fn display_name(&self) -> String {
        format!("Video {}", self.model_type())
    }
}
