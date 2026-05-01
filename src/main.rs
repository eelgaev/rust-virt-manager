mod app;
mod backend;
mod config;
mod connection;
mod console;
mod devices;
mod domain;
mod edit_state;
mod error;
mod qemu_capabilities;
mod state;
mod uri;
mod views;
mod widgets;
mod xml_helpers;

use app::VirtManagerApp;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Virtual Machine Manager")
            .with_inner_size([550.0, 550.0]),
        ..Default::default()
    };

    eframe::run_native(
        "rust-virt-manager",
        options,
        Box::new(|cc| Ok(Box::new(VirtManagerApp::new(cc)))),
    )
}
