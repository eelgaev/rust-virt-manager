use thiserror::Error;

#[derive(Debug, Error)]
pub enum VirtError {
    #[error("libvirt error: {0}")]
    Libvirt(#[from] virt::error::Error),

    #[error("XML parsing error: {0}")]
    Xml(#[from] quick_xml::DeError),

    #[error("invalid URI: {0}")]
    InvalidUri(String),

    #[error("not connected")]
    NotConnected,

    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, VirtError>;
