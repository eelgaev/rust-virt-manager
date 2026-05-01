use egui::{RichText, Ui, ViewportBuilder, ViewportId};

use crate::backend::BackendCommand;
use crate::state::AppState;

const FORWARD_MODES: [&str; 4] = ["nat", "route", "open", "isolated"];

pub struct CreateNetworkState {
    pub open: bool,
    pub uri: String,
    pub name: String,
    pub forward_mode: usize,
    pub domain_name: String,
    pub enable_ipv4: bool,
    pub ipv4_network: String,
    pub enable_dhcp: bool,
    pub dhcp_start: String,
    pub dhcp_end: String,
    pub enable_ipv6: bool,
    pub ipv6_network: String,
    pub ipv6_prefix: String,
    pub enable_dhcpv6: bool,
    pub dhcpv6_start: String,
    pub dhcpv6_end: String,
    pub dns_forwarders: Vec<String>,
    pub dns_forwarder_input: String,
    pub port_forwards: Vec<PortForward>,
}

#[derive(Clone)]
pub struct PortForward {
    pub protocol: String,
    pub host_port: String,
    pub guest_ip: String,
    pub guest_port: String,
}

impl Default for CreateNetworkState {
    fn default() -> Self {
        Self {
            open: false,
            uri: String::new(),
            name: String::new(),
            forward_mode: 0,
            domain_name: String::new(),
            enable_ipv4: true,
            ipv4_network: "192.168.100.0/24".into(),
            enable_dhcp: true,
            dhcp_start: "192.168.100.128".into(),
            dhcp_end: "192.168.100.254".into(),
            enable_ipv6: false,
            ipv6_network: "fd00::1".into(),
            ipv6_prefix: "64".into(),
            enable_dhcpv6: false,
            dhcpv6_start: "fd00::100".into(),
            dhcpv6_end: "fd00::1ff".into(),
            dns_forwarders: Vec::new(),
            dns_forwarder_input: String::new(),
            port_forwards: Vec::new(),
        }
    }
}

pub fn show_create_network_window(ctx: &egui::Context, state: &mut AppState) {
    if !state.create_network.open {
        return;
    }

    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("create_network"),
        ViewportBuilder::default()
            .with_title("Create Virtual Network")
            .with_inner_size([500.0, 550.0]),
        |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                state.create_network = CreateNetworkState::default();
            }
            egui::CentralPanel::default().show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    show_network_form(ui, state);
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("✅ Create").clicked() {
                        create_network(state);
                        state.create_network = CreateNetworkState::default();
                    }
                    if ui.button("Cancel").clicked() {
                        state.create_network = CreateNetworkState::default();
                    }
                });
            });
        },
    );
}

fn show_network_form(ui: &mut Ui, state: &mut AppState) {
    ui.heading("Network Configuration");
    ui.add_space(8.0);

    egui::Grid::new("net_form_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut state.create_network.name);
            ui.end_row();

            ui.label("Forward Mode:");
            egui::ComboBox::from_id_salt("net_fwd_mode")
                .selected_text(FORWARD_MODES[state.create_network.forward_mode])
                .show_ui(ui, |ui| {
                    for (i, m) in FORWARD_MODES.iter().enumerate() {
                        ui.selectable_value(&mut state.create_network.forward_mode, i, *m);
                    }
                });
            ui.end_row();

            ui.label("Domain name:");
            ui.text_edit_singleline(&mut state.create_network.domain_name);
            ui.end_row();
        });

    ui.add_space(8.0);
    ui.label(RichText::new("IPv4").strong());
    ui.separator();

    egui::Grid::new("net_ipv4_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Enable IPv4:");
            ui.checkbox(&mut state.create_network.enable_ipv4, "");
            ui.end_row();

            if state.create_network.enable_ipv4 {
                ui.label("Network (CIDR):");
                ui.text_edit_singleline(&mut state.create_network.ipv4_network);
                ui.end_row();

                ui.label("Enable DHCP:");
                ui.checkbox(&mut state.create_network.enable_dhcp, "");
                ui.end_row();

                if state.create_network.enable_dhcp {
                    ui.label("DHCP Start:");
                    ui.text_edit_singleline(&mut state.create_network.dhcp_start);
                    ui.end_row();

                    ui.label("DHCP End:");
                    ui.text_edit_singleline(&mut state.create_network.dhcp_end);
                    ui.end_row();
                }
            }
        });

    ui.add_space(8.0);
    ui.label(RichText::new("IPv6").strong());
    ui.separator();

    egui::Grid::new("net_ipv6_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Enable IPv6:");
            ui.checkbox(&mut state.create_network.enable_ipv6, "");
            ui.end_row();

            if state.create_network.enable_ipv6 {
                ui.label("Address:");
                ui.text_edit_singleline(&mut state.create_network.ipv6_network);
                ui.end_row();

                ui.label("Prefix:");
                ui.text_edit_singleline(&mut state.create_network.ipv6_prefix);
                ui.end_row();

                ui.label("Enable DHCPv6:");
                ui.checkbox(&mut state.create_network.enable_dhcpv6, "");
                ui.end_row();

                if state.create_network.enable_dhcpv6 {
                    ui.label("DHCPv6 Start:");
                    ui.text_edit_singleline(&mut state.create_network.dhcpv6_start);
                    ui.end_row();

                    ui.label("DHCPv6 End:");
                    ui.text_edit_singleline(&mut state.create_network.dhcpv6_end);
                    ui.end_row();
                }
            }
        });

    ui.add_space(8.0);
    ui.label(RichText::new("DNS").strong());
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Forwarder:");
        ui.text_edit_singleline(&mut state.create_network.dns_forwarder_input);
        if ui.button("➕ Add").clicked() {
            let addr = state.create_network.dns_forwarder_input.trim().to_string();
            if !addr.is_empty() {
                state.create_network.dns_forwarders.push(addr);
                state.create_network.dns_forwarder_input.clear();
            }
        }
    });

    let mut to_remove = None;
    for (i, fwd) in state.create_network.dns_forwarders.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("  {fwd}"));
            if ui.small_button("✕").clicked() {
                to_remove = Some(i);
            }
        });
    }
    if let Some(i) = to_remove {
        state.create_network.dns_forwarders.remove(i);
    }

    if state.create_network.forward_mode == 0 {
        ui.add_space(8.0);
        ui.label(RichText::new("Port Forwarding (NAT only)").strong());
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("➕ Add Rule").clicked() {
                state.create_network.port_forwards.push(PortForward {
                    protocol: "tcp".into(),
                    host_port: String::new(),
                    guest_ip: String::new(),
                    guest_port: String::new(),
                });
            }
        });

        let mut pf_to_remove = None;
        for (i, pf) in state.create_network.port_forwards.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt(format!("pf_proto_{i}"))
                    .selected_text(&pf.protocol)
                    .width(50.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut pf.protocol, "tcp".into(), "tcp");
                        ui.selectable_value(&mut pf.protocol, "udp".into(), "udp");
                    });
                ui.label("host:");
                ui.add(egui::TextEdit::singleline(&mut pf.host_port).desired_width(50.0));
                ui.label("->");
                ui.add(egui::TextEdit::singleline(&mut pf.guest_ip).desired_width(90.0));
                ui.label(":");
                ui.add(egui::TextEdit::singleline(&mut pf.guest_port).desired_width(50.0));
                if ui.small_button("✕").clicked() {
                    pf_to_remove = Some(i);
                }
            });
        }
        if let Some(i) = pf_to_remove {
            state.create_network.port_forwards.remove(i);
        }
    }
}

fn create_network(state: &mut AppState) {
    let n = &state.create_network;
    let mode = FORWARD_MODES[n.forward_mode];

    let forward_xml = if mode == "isolated" {
        String::new()
    } else {
        let port_rules: String = n.port_forwards.iter()
            .filter(|pf| !pf.host_port.is_empty() && !pf.guest_port.is_empty())
            .map(|pf| {
                let addr = if pf.guest_ip.is_empty() {
                    String::new()
                } else {
                    format!(" address='{}'", pf.guest_ip)
                };
                format!(
                    "<nat><port start='{}' end='{}'/></nat>",
                    pf.host_port, pf.guest_port,
                ) + &format!(
                    "<!-- {proto} {host}->{addr}:{guest} -->",
                    proto = pf.protocol,
                    host = pf.host_port,
                    guest = pf.guest_port,
                )
            })
            .collect::<Vec<_>>()
            .join("");

        if mode == "nat" && !port_rules.is_empty() {
            format!("<forward mode='{mode}'>{port_rules}</forward>")
        } else {
            format!("<forward mode='{mode}'/>")
        }
    };

    let domain_xml = if n.domain_name.trim().is_empty() {
        String::new()
    } else {
        format!("<domain name='{}'/>", n.domain_name.trim())
    };

    let dns_xml = if n.dns_forwarders.is_empty() {
        String::new()
    } else {
        let entries: String = n.dns_forwarders.iter()
            .map(|f| format!("<forwarder addr='{f}'/>"))
            .collect::<Vec<_>>()
            .join("");
        format!("<dns>{entries}</dns>")
    };

    let ipv4_xml = if n.enable_ipv4 {
        let parts: Vec<&str> = n.ipv4_network.split('/').collect();
        let addr = parts.first().copied().unwrap_or("192.168.100.0");
        let prefix = parts.get(1).copied().unwrap_or("24");

        let prefix_num: u32 = prefix.parse().unwrap_or(24);
        let netmask = prefix_to_netmask(prefix_num);

        let dhcp = if n.enable_dhcp {
            format!(
                "<dhcp><range start='{}' end='{}'/></dhcp>",
                n.dhcp_start, n.dhcp_end
            )
        } else {
            String::new()
        };

        let gateway = addr.rsplit_once('.').map_or(addr.to_string(), |(base, _)| {
            format!("{base}.1")
        });

        format!("<ip address='{gateway}' netmask='{netmask}'>{dhcp}</ip>")
    } else {
        String::new()
    };

    let ipv6_xml = if n.enable_ipv6 {
        let dhcpv6 = if n.enable_dhcpv6 {
            format!(
                "<dhcp><range start='{}' end='{}'/></dhcp>",
                n.dhcpv6_start, n.dhcpv6_end
            )
        } else {
            String::new()
        };
        format!(
            "<ip family='ipv6' address='{}' prefix='{}'>{dhcpv6}</ip>",
            n.ipv6_network, n.ipv6_prefix
        )
    } else {
        String::new()
    };

    let xml = format!(
        "<network>\
           <name>{}</name>\
           {forward_xml}\
           {domain_xml}\
           {dns_xml}\
           {ipv4_xml}\
           {ipv6_xml}\
         </network>",
        n.name,
    );

    let uri = n.uri.clone();
    state
        .backend
        .send_to(&uri, BackendCommand::CreateNetwork(uri.clone(), xml));
}

fn prefix_to_netmask(prefix: u32) -> String {
    let mask: u32 = if prefix >= 32 {
        0xFFFFFFFF
    } else {
        !((1u32 << (32 - prefix)) - 1)
    };
    format!(
        "{}.{}.{}.{}",
        (mask >> 24) & 0xFF,
        (mask >> 16) & 0xFF,
        (mask >> 8) & 0xFF,
        mask & 0xFF
    )
}
