use crate::error::{Result, VirtError};

#[derive(Debug, Clone)]
pub struct LibvirtUri {
    pub raw: String,
    pub driver: String,
    pub transport: String,
    pub hostname: String,
    pub port: Option<u16>,
    pub path: String,
    pub username: Option<String>,
}

impl LibvirtUri {
    pub fn parse(uri: &str) -> Result<Self> {
        let url = url::Url::parse(uri)
            .map_err(|e| VirtError::InvalidUri(format!("{uri}: {e}")))?;

        let scheme = url.scheme();
        let (driver, transport) = match scheme.split_once('+') {
            Some((d, t)) => (d.to_string(), t.to_string()),
            None => (scheme.to_string(), String::new()),
        };

        let hostname = url.host_str().unwrap_or("").to_string();
        let port = url.port();
        let path = url.path().to_string();
        let username = if url.username().is_empty() {
            None
        } else {
            Some(url.username().to_string())
        };

        Ok(LibvirtUri {
            raw: uri.to_string(),
            driver,
            transport,
            hostname,
            port,
            path,
            username,
        })
    }

    pub fn is_remote(&self) -> bool {
        !self.hostname.is_empty()
    }

    pub fn is_session(&self) -> bool {
        self.path == "/session"
    }

    pub fn display_name(&self) -> String {
        let driver_label = match (self.driver.as_str(), self.path.as_str()) {
            ("qemu", "/session") => "QEMU/KVM user session",
            ("qemu", _) => "QEMU/KVM",
            ("xen", _) => "Xen",
            ("lxc", _) => "LXC",
            ("vbox", _) => "VirtualBox",
            _ => &self.driver,
        };

        if self.is_remote() {
            format!("{} ({})", self.hostname, driver_label)
        } else {
            driver_label.to_string()
        }
    }
}

impl std::fmt::Display for LibvirtUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_local_system() {
        let uri = LibvirtUri::parse("qemu:///system").unwrap();
        assert_eq!(uri.driver, "qemu");
        assert_eq!(uri.transport, "");
        assert_eq!(uri.hostname, "");
        assert_eq!(uri.path, "/system");
        assert!(!uri.is_remote());
    }

    #[test]
    fn parse_remote_ssh() {
        let uri = LibvirtUri::parse("qemu+ssh://user@server.example.com/system").unwrap();
        assert_eq!(uri.driver, "qemu");
        assert_eq!(uri.transport, "ssh");
        assert_eq!(uri.hostname, "server.example.com");
        assert_eq!(uri.username, Some("user".to_string()));
        assert_eq!(uri.path, "/system");
        assert!(uri.is_remote());
    }

    #[test]
    fn parse_session() {
        let uri = LibvirtUri::parse("qemu:///session").unwrap();
        assert!(uri.is_session());
    }
}
