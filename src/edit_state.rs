use std::collections::HashMap;
use crate::domain::Guest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSubTab {
    Details,
    Xml,
}

pub struct DeviceEditState<T> {
    pub fields: T,
    pub dirty: bool,
    pub sub_tab: DeviceSubTab,
    pub device_xml_text: String,
    pub device_xml_dirty: bool,
}

impl<T> DeviceEditState<T> {
    pub fn new(fields: T) -> Self {
        Self {
            fields,
            dirty: false,
            sub_tab: DeviceSubTab::Details,
            device_xml_text: String::new(),
            device_xml_dirty: false,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty || self.device_xml_dirty
    }
}

pub struct OverviewEdit {
    pub name: String,
    pub title: String,
    pub description: String,
}

impl OverviewEdit {
    pub fn from_guest(guest: &Guest) -> Self {
        Self {
            name: guest.name.clone(),
            title: guest.title.clone().unwrap_or_default(),
            description: guest.description.clone().unwrap_or_default(),
        }
    }

    pub fn to_xml_parts(&self) -> Vec<(&str, String)> {
        let mut parts = vec![("name", format!("<name>{}</name>", self.name))];
        if !self.title.is_empty() {
            parts.push(("title", format!("<title>{}</title>", self.title)));
        }
        if !self.description.is_empty() {
            parts.push(("description", format!("<description>{}</description>", self.description)));
        }
        parts
    }
}

pub struct CpuEdit {
    pub vcpu_count: u32,
    pub current: u32,
    pub has_current: bool,
}

impl CpuEdit {
    pub fn from_guest(guest: &Guest) -> Self {
        Self {
            vcpu_count: guest.vcpu.count,
            current: guest.vcpu.current.unwrap_or(guest.vcpu.count),
            has_current: guest.vcpu.current.is_some(),
        }
    }

    pub fn to_xml(&self) -> String {
        if self.has_current && self.current != self.vcpu_count {
            format!("<vcpu current='{}'>{}</vcpu>", self.current, self.vcpu_count)
        } else {
            format!("<vcpu>{}</vcpu>", self.vcpu_count)
        }
    }
}

pub struct MemoryEdit {
    pub max_memory_mib: u64,
    pub current_memory_mib: u64,
}

impl MemoryEdit {
    pub fn from_guest(guest: &Guest) -> Self {
        Self {
            max_memory_mib: guest.memory.as_mib(),
            current_memory_mib: guest
                .current_memory
                .as_ref()
                .map(|m| m.as_mib())
                .unwrap_or(guest.memory.as_mib()),
        }
    }

    pub fn to_xml(&self) -> (String, String) {
        let mem = format!("<memory unit='MiB'>{}</memory>", self.max_memory_mib);
        let cur = format!(
            "<currentMemory unit='MiB'>{}</currentMemory>",
            self.current_memory_mib
        );
        (mem, cur)
    }
}

pub struct BootEdit {
    pub boot_devices: Vec<String>,
}

impl BootEdit {
    pub fn from_guest(guest: &Guest) -> Self {
        Self {
            boot_devices: guest.os.boot.iter().map(|b| b.dev.clone()).collect(),
        }
    }
}

pub struct DiskEdit {
    pub source_path: String,
    pub target_dev: String,
    pub bus: String,
    pub cache: String,
    pub format: String,
    pub device_type: String,
    pub disk_type: String,
    pub readonly: bool,
}

impl DiskEdit {
    pub fn from_guest(guest: &Guest, idx: usize) -> Option<Self> {
        let disk = guest.devices.as_ref()?.disks.get(idx)?;
        Some(Self {
            source_path: disk.source_path().unwrap_or("").to_string(),
            target_dev: disk.target.dev.clone(),
            bus: disk.target.bus.clone().unwrap_or_default(),
            cache: disk
                .driver
                .as_ref()
                .and_then(|d| d.cache.clone())
                .unwrap_or_default(),
            format: disk
                .driver
                .as_ref()
                .and_then(|d| d.driver_type.clone())
                .unwrap_or_default(),
            device_type: disk.device.clone(),
            disk_type: disk.disk_type.clone(),
            readonly: disk.readonly.is_some(),
        })
    }

    pub fn to_xml(&self) -> String {
        let mut xml = format!("<disk type='{}' device='{}'>", self.disk_type, self.device_type);
        if !self.format.is_empty() {
            xml.push_str(&format!(
                "\n      <driver name='qemu' type='{}'{}/>",
                self.format,
                if self.cache.is_empty() {
                    String::new()
                } else {
                    format!(" cache='{}'", self.cache)
                }
            ));
        }
        if !self.source_path.is_empty() {
            if self.disk_type == "block" {
                xml.push_str(&format!("\n      <source dev='{}'/>", self.source_path));
            } else {
                xml.push_str(&format!("\n      <source file='{}'/>", self.source_path));
            }
        }
        xml.push_str(&format!("\n      <target dev='{}'", self.target_dev));
        if !self.bus.is_empty() {
            xml.push_str(&format!(" bus='{}'", self.bus));
        }
        xml.push_str("/>");
        if self.readonly {
            xml.push_str("\n      <readonly/>");
        }
        xml.push_str("\n    </disk>");
        xml
    }
}

pub struct NicEdit {
    pub interface_type: String,
    pub source: String,
    pub model: String,
    pub mac_address: String,
}

impl NicEdit {
    pub fn from_guest(guest: &Guest, idx: usize) -> Option<Self> {
        let nic = guest.devices.as_ref()?.interfaces.get(idx)?;
        let source = nic
            .source
            .as_ref()
            .and_then(|s| {
                s.network
                    .as_ref()
                    .or(s.bridge.as_ref())
                    .or(s.dev.as_ref())
            })
            .cloned()
            .unwrap_or_default();
        Some(Self {
            interface_type: nic.interface_type.clone(),
            source,
            model: nic.model.as_ref().map(|m| m.model_type.clone()).unwrap_or_default(),
            mac_address: nic.mac.as_ref().map(|m| m.address.clone()).unwrap_or_default(),
        })
    }

    pub fn to_xml(&self) -> String {
        let mut xml = format!("<interface type='{}'>", self.interface_type);
        if !self.mac_address.is_empty() {
            xml.push_str(&format!("\n      <mac address='{}'/>", self.mac_address));
        }
        if !self.source.is_empty() {
            let attr = match self.interface_type.as_str() {
                "bridge" => format!("bridge='{}'", self.source),
                "direct" => format!("dev='{}'", self.source),
                _ => format!("network='{}'", self.source),
            };
            xml.push_str(&format!("\n      <source {attr}/>"));
        }
        if !self.model.is_empty() {
            xml.push_str(&format!("\n      <model type='{}'/>", self.model));
        }
        xml.push_str("\n    </interface>");
        xml
    }
}

pub struct GraphicsEdit {
    pub graphics_type: String,
    pub port: i32,
    pub listen: String,
    pub autoport: bool,
    pub password: String,
    pub keymap: String,
}

impl GraphicsEdit {
    pub fn from_guest(guest: &Guest, idx: usize) -> Option<Self> {
        let gfx = guest.devices.as_ref()?.graphics.get(idx)?;
        Some(Self {
            graphics_type: gfx.graphics_type.clone(),
            port: gfx.port.unwrap_or(-1),
            listen: gfx.listen.clone().unwrap_or_default(),
            autoport: gfx.autoport.as_deref() == Some("yes"),
            password: gfx.passwd.clone().unwrap_or_default(),
            keymap: gfx.keymap.clone().unwrap_or_default(),
        })
    }

    pub fn to_xml(&self) -> String {
        let mut xml = format!("<graphics type='{}'", self.graphics_type);
        if self.port >= 0 {
            xml.push_str(&format!(" port='{}'", self.port));
        }
        xml.push_str(&format!(
            " autoport='{}'",
            if self.autoport { "yes" } else { "no" }
        ));
        if !self.listen.is_empty() {
            xml.push_str(&format!(" listen='{}'", self.listen));
        }
        if !self.password.is_empty() {
            xml.push_str(&format!(" passwd='{}'", self.password));
        }
        if !self.keymap.is_empty() {
            xml.push_str(&format!(" keymap='{}'", self.keymap));
        }
        xml.push_str("/>");
        xml
    }
}

pub struct VideoEdit {
    pub model_type: String,
    pub vram_kib: u64,
    pub heads: u32,
    pub accel3d: bool,
}

impl VideoEdit {
    pub fn from_guest(guest: &Guest, idx: usize) -> Option<Self> {
        let vid = guest.devices.as_ref()?.videos.get(idx)?;
        Some(Self {
            model_type: vid.model.model_type.clone().unwrap_or_default(),
            vram_kib: vid.model.vram.unwrap_or(16384),
            heads: vid.model.heads.unwrap_or(1),
            accel3d: vid
                .model
                .acceleration
                .as_ref()
                .and_then(|a| a.accel3d.as_deref())
                == Some("yes"),
        })
    }

    pub fn to_xml(&self) -> String {
        let mut xml = format!(
            "<video>\n      <model type='{}' vram='{}' heads='{}'",
            self.model_type, self.vram_kib, self.heads
        );
        if self.accel3d {
            xml.push_str(">\n        <acceleration accel3d='yes'/>\n      </model>");
        } else {
            xml.push_str("/>");
        }
        xml.push_str("\n    </video>");
        xml
    }
}

pub struct SoundEdit {
    pub model: String,
}

impl SoundEdit {
    pub fn from_guest(guest: &Guest, idx: usize) -> Option<Self> {
        let snd = guest.devices.as_ref()?.sounds.get(idx)?;
        Some(Self {
            model: snd.model.clone(),
        })
    }

    pub fn to_xml(&self) -> String {
        format!("<sound model='{}'/>", self.model)
    }
}

#[derive(Default)]
pub struct DeviceEdits {
    pub overview: Option<DeviceEditState<OverviewEdit>>,
    pub cpu: Option<DeviceEditState<CpuEdit>>,
    pub memory: Option<DeviceEditState<MemoryEdit>>,
    pub boot: Option<DeviceEditState<BootEdit>>,
    pub disk: HashMap<usize, DeviceEditState<DiskEdit>>,
    pub nic: HashMap<usize, DeviceEditState<NicEdit>>,
    pub graphics: HashMap<usize, DeviceEditState<GraphicsEdit>>,
    pub video: HashMap<usize, DeviceEditState<VideoEdit>>,
    pub sound: HashMap<usize, DeviceEditState<SoundEdit>>,
}

impl DeviceEdits {
    pub fn clear_clean(&mut self) {
        if let Some(ref e) = self.overview {
            if !e.is_dirty() {
                self.overview = None;
            }
        }
        if let Some(ref e) = self.cpu {
            if !e.is_dirty() {
                self.cpu = None;
            }
        }
        if let Some(ref e) = self.memory {
            if !e.is_dirty() {
                self.memory = None;
            }
        }
        if let Some(ref e) = self.boot {
            if !e.is_dirty() {
                self.boot = None;
            }
        }
        self.disk.retain(|_, e| e.is_dirty());
        self.nic.retain(|_, e| e.is_dirty());
        self.graphics.retain(|_, e| e.is_dirty());
        self.video.retain(|_, e| e.is_dirty());
        self.sound.retain(|_, e| e.is_dirty());
    }
}
