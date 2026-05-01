use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct QemuCapabilities {
    pub loaded: bool,
    pub video_models: Vec<String>,
    pub sound_models: Vec<String>,
    pub nic_models: Vec<String>,
    pub disk_buses: Vec<String>,
    pub machine_types: Vec<String>,
}

impl QemuCapabilities {
    pub fn query(arch: &str) -> Self {
        let binary = format!("qemu-system-{arch}");
        let mut caps = Self {
            loaded: true,
            disk_buses: vec![
                "virtio".into(),
                "scsi".into(),
                "sata".into(),
                "ide".into(),
                "usb".into(),
            ],
            ..Default::default()
        };

        if let Ok(output) = Command::new(&binary).args(["-vga", "help"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let name = line.trim();
                if !name.is_empty() && name != "Valid values for -vga:" {
                    caps.video_models.push(name.to_string());
                }
            }
        }
        if caps.video_models.is_empty() {
            caps.video_models = vec![
                "virtio".into(),
                "qxl".into(),
                "vga".into(),
                "bochs".into(),
                "cirrus".into(),
                "vmware".into(),
                "none".into(),
            ];
        }

        if let Ok(output) = Command::new(&binary).args(["-device", "help"]).output() {
            let text = String::from_utf8_lossy(&output.stderr);
            parse_device_help(&text, &mut caps);
        }

        if caps.sound_models.is_empty() {
            caps.sound_models = vec![
                "ich9".into(),
                "ich6".into(),
                "ac97".into(),
                "es1370".into(),
                "sb16".into(),
                "usb-audio".into(),
            ];
        }
        if caps.nic_models.is_empty() {
            caps.nic_models = vec![
                "virtio".into(),
                "e1000".into(),
                "e1000e".into(),
                "rtl8139".into(),
                "vmxnet3".into(),
            ];
        }

        if let Ok(output) = Command::new(&binary).args(["-machine", "help"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if let Some(name) = line.split_whitespace().next() {
                    if !name.is_empty()
                        && name != "Supported"
                        && name != "supported"
                        && !name.starts_with('-')
                    {
                        caps.machine_types.push(name.to_string());
                    }
                }
            }
        }

        caps
    }

    pub fn fallback() -> Self {
        Self {
            loaded: false,
            video_models: vec![
                "virtio".into(), "qxl".into(), "vga".into(), "bochs".into(),
                "cirrus".into(), "vmware".into(), "none".into(),
            ],
            sound_models: vec![
                "ich9".into(), "ich6".into(), "ac97".into(), "es1370".into(),
                "sb16".into(), "usb-audio".into(),
            ],
            nic_models: vec![
                "virtio".into(), "e1000".into(), "e1000e".into(),
                "rtl8139".into(), "vmxnet3".into(),
            ],
            disk_buses: vec![
                "virtio".into(), "scsi".into(), "sata".into(),
                "ide".into(), "usb".into(),
            ],
            machine_types: Vec::new(),
        }
    }
}

fn parse_device_help(text: &str, caps: &mut QemuCapabilities) {
    let mut in_sound = false;
    let mut in_network = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Sound") {
            in_sound = true;
            in_network = false;
            continue;
        }
        if trimmed.starts_with("Network") {
            in_network = true;
            in_sound = false;
            continue;
        }
        if !trimmed.is_empty()
            && !trimmed.starts_with("name")
            && trimmed.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && !trimmed.starts_with("Sound")
            && !trimmed.starts_with("Network")
        {
            in_sound = false;
            in_network = false;
        }

        if (in_sound || in_network) && trimmed.starts_with("name") {
            if let Some(name) = trimmed
                .strip_prefix("name \"")
                .and_then(|s| s.split('"').next())
            {
                if in_sound {
                    caps.sound_models.push(name.to_string());
                } else {
                    caps.nic_models.push(name.to_string());
                }
            }
        }
    }
}
