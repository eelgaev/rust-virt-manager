use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub saved_uris: Vec<String>,
    pub auto_connect: bool,
    pub stats_update_interval_secs: u64,
    pub show_guest_cpu: bool,
    pub show_host_cpu: bool,
    pub show_memory: bool,
    pub show_disk_io: bool,
    pub show_network_io: bool,
    pub window_width: f32,
    pub window_height: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            saved_uris: if cfg!(target_os = "linux") {
                vec!["qemu:///system".to_string()]
            } else {
                vec![]
            },
            auto_connect: false,
            stats_update_interval_secs: 2,
            show_guest_cpu: true,
            show_host_cpu: false,
            show_memory: true,
            show_disk_io: true,
            show_network_io: true,
            window_width: 550.0,
            window_height: 550.0,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        confy::load("rust-virt-manager", "config").unwrap_or_default()
    }

    pub fn save(&self) {
        if let Err(e) = confy::store("rust-virt-manager", "config", self) {
            log::error!("Failed to save config: {e}");
        }
    }
}
