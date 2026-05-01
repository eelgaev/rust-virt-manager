use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFilesystem {
    #[serde(rename = "@type", default)]
    pub fs_type: Option<String>,

    #[serde(rename = "@accessmode", default)]
    pub accessmode: Option<String>,

    #[serde(rename = "@model", default)]
    pub model: Option<String>,

    #[serde(default)]
    pub source: Option<FilesystemSource>,

    #[serde(default)]
    pub target: Option<FilesystemTarget>,

    #[serde(default)]
    pub driver: Option<FilesystemDriver>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemSource {
    #[serde(rename = "@dir", default)]
    pub dir: Option<String>,

    #[serde(rename = "@name", default)]
    pub name: Option<String>,

    #[serde(rename = "@socket", default)]
    pub socket: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemTarget {
    #[serde(rename = "@dir")]
    pub dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemDriver {
    #[serde(rename = "@type", default)]
    pub driver_type: Option<String>,

    #[serde(rename = "@wrpolicy", default)]
    pub wrpolicy: Option<String>,
}

impl DeviceFilesystem {
    pub fn display_name(&self) -> String {
        if let Some(target) = &self.target {
            return format!("Filesystem {}", target.dir);
        }
        "Filesystem".to_string()
    }
}
