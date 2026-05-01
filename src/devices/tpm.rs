use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTpm {
    #[serde(rename = "@model", default)]
    pub model: Option<String>,

    #[serde(default)]
    pub backend: Option<TpmBackend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpmBackend {
    #[serde(rename = "@type")]
    pub backend_type: String,

    #[serde(rename = "@version", default)]
    pub version: Option<String>,

    #[serde(default)]
    pub device: Option<TpmDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpmDevice {
    #[serde(rename = "@path", default)]
    pub path: Option<String>,
}

impl DeviceTpm {
    pub fn display_name(&self) -> String {
        if let Some(backend) = &self.backend {
            if let Some(ver) = &backend.version {
                return format!("TPM v{ver}");
            }
        }
        "TPM".to_string()
    }
}
