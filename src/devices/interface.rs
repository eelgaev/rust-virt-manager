use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInterface {
    #[serde(rename = "@type")]
    pub interface_type: String,

    #[serde(default)]
    pub mac: Option<MacAddress>,

    #[serde(default)]
    pub source: Option<InterfaceSource>,

    #[serde(default)]
    pub model: Option<InterfaceModel>,

    #[serde(default)]
    pub address: Option<InterfaceAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacAddress {
    #[serde(rename = "@address")]
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceSource {
    #[serde(rename = "@network", default)]
    pub network: Option<String>,

    #[serde(rename = "@bridge", default)]
    pub bridge: Option<String>,

    #[serde(rename = "@dev", default)]
    pub dev: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceModel {
    #[serde(rename = "@type")]
    pub model_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceAddress {
    #[serde(rename = "@type")]
    pub address_type: String,
}

impl DeviceInterface {
    pub fn source_name(&self) -> Option<&str> {
        self.source.as_ref().and_then(|s| {
            s.network.as_deref().or(s.bridge.as_deref()).or(s.dev.as_deref())
        })
    }

    pub fn model_type(&self) -> &str {
        self.model
            .as_ref()
            .map(|m| m.model_type.as_str())
            .unwrap_or("unknown")
    }

    pub fn display_name(&self) -> String {
        let source = self.source_name().unwrap_or("unknown");
        format!("NIC {} ({})", source, self.model_type())
    }
}
