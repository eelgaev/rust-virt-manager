use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceVsock {
    #[serde(rename = "@model", default)]
    pub model: Option<String>,

    #[serde(default)]
    pub cid: Option<VsockCid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsockCid {
    #[serde(rename = "@auto", default)]
    pub auto: Option<String>,

    #[serde(rename = "@address", default)]
    pub address: Option<String>,
}

impl DeviceVsock {
    pub fn display_name(&self) -> String {
        "VSOCK".to_string()
    }
}
