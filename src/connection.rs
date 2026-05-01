use virt::connect::Connect;

use crate::error::{Result, VirtError};
use crate::uri::LibvirtUri;

pub struct LibvirtConnection {
    uri: LibvirtUri,
    conn: Option<Connect>,
}

impl LibvirtConnection {
    pub fn new(uri: LibvirtUri) -> Self {
        Self { uri, conn: None }
    }

    pub fn open(&mut self) -> Result<()> {
        let conn = Connect::open(Some(&self.uri.raw))?;
        self.conn = Some(conn);
        Ok(())
    }

    pub fn open_read_only(&mut self) -> Result<()> {
        let conn = Connect::open_read_only(Some(&self.uri.raw))?;
        self.conn = Some(conn);
        Ok(())
    }

    pub fn close(&mut self) {
        if let Some(mut conn) = self.conn.take() {
            let _ = conn.close();
        }
    }

    pub fn is_connected(&self) -> bool {
        self.conn.is_some()
    }

    pub fn uri(&self) -> &LibvirtUri {
        &self.uri
    }

    pub fn conn(&self) -> Result<&Connect> {
        self.conn.as_ref().ok_or(VirtError::NotConnected)
    }

    pub fn hostname(&self) -> Result<String> {
        let conn = self.conn()?;
        Ok(conn.get_hostname()?)
    }

    pub fn lib_version(&self) -> Result<u32> {
        let conn = self.conn()?;
        Ok(conn.get_lib_version()?)
    }

    pub fn list_domains(&self) -> Result<Vec<virt::domain::Domain>> {
        let conn = self.conn()?;
        Ok(conn.list_all_domains(0)?)
    }

    pub fn list_networks(&self) -> Result<Vec<virt::network::Network>> {
        let conn = self.conn()?;
        Ok(conn.list_all_networks(0)?)
    }

    pub fn list_storage_pools(&self) -> Result<Vec<virt::storage_pool::StoragePool>> {
        let conn = self.conn()?;
        Ok(conn.list_all_storage_pools(0)?)
    }

    pub fn list_node_devices(&self) -> Result<Vec<virt::nodedev::NodeDevice>> {
        let conn = self.conn()?;
        Ok(conn.list_all_node_devices(0)?)
    }
}

impl Drop for LibvirtConnection {
    fn drop(&mut self) {
        self.close();
    }
}
