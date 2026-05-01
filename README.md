# rust-virt-manager

> **WARNING!** This project is vibe coded, use at your own risk.

A fast, modern replacement for [virt-manager](https://virt-manager.org/), built in Rust with [egui](https://www.egui.rs/).

virt-manager is unmaintained, built on Python/GTK3, and slow. This project reimplements it from scratch with the same UI layout and feature set, but significantly better performance.

**Linux only.** This manages KVM/QEMU virtual machines via libvirt. If you're not on Linux, this isn't for you.

## What's different

- **Faster polling** -- bulk `getAllDomainStats()` API instead of per-VM calls
- **Event-driven** -- proper libvirt event callbacks for instant state updates
- **Per-connection threading** -- remote connections don't block each other
- **Native code** -- no Python GIL, no GLib marshaling overhead

## Goals

Reach feature parity with virt-manager for day-to-day KVM/QEMU management, without the Python/GTK baggage.

### What works

- VM lifecycle -- start, stop, pause, resume, reboot, force off
- VNC console -- built-in viewer with keyboard/mouse input and Send Key menu
- Remote connections -- SSH tunneling for both libvirt and VNC
- Serial console -- in-app terminal emulator
- Device editing -- CPU, memory, disk, NIC, graphics, video, sound, boot order (Details/XML sub-tabs per device, apply/revert per device)
- Host/connection overview -- hostname, CPU, memory, architecture, libvirt version
- VM creation wizard -- OS presets, ISO/import/manual install, storage pool selection
- Cloning and migration
- Storage management -- pool create/start/stop/delete, volume create/delete, volume browser
- Network management -- create/delete virtual networks, IPv4/IPv6, DHCP, DNS, NAT port forwarding
- Add hardware -- disk, NIC, graphics, video, sound, input, watchdog, filesystem, TPM, RNG, serial
- Performance monitoring -- live CPU, memory, disk I/O, network I/O graphs
- QEMU capability detection -- hardware option dropdowns populated from `qemu-system-*`

### What's missing

- Reliable VM configuration changes (editing devices, adding hardware, and XML modifications may not apply correctly)
- SPICE display protocol (VNC only for now)
- Snapshots UI (backend commands exist, tab is disabled)
- Apply/revert and XML editing for networks and storage pools in host properties
- Storage pool types: fs, iscsi, scsi, mpath, gluster, rbd, zfs (only dir, logical, netfs, disk supported)
- VNC display scaling options (fit-to-window, zoom, fullscreen)
- USB/PCI host device passthrough
- USB redirection (SPICE)
- Guest agent integration (IP addresses, filesystem info)
- LXC, Bhyve, and Virtuozzo connectors (only libvirt QEMU/KVM and Xen are wired up)
- CPU topology (sockets, cores, threads) and pinning
- Firmware selection (BIOS/UEFI picker in details)
- Machine type selection (pc/q35)
- libosinfo-based OS detection from install media
- System tray
- Screenshot capture
- Console auto-resize guest

## Status

Early development. Core VM management, VNC console, storage, and networking are functional.

## Dependencies

Build dependencies:

- Rust 1.85+ (edition 2024)
- `libvirt-dev` / `libvirt-devel` (libvirt C headers)
- `pkg-config`

Runtime dependencies:

- `libvirt`
- `openssh` (system `ssh` binary for remote connections)
- A running `libvirtd`


## Building

```sh
cargo build --release
```

## License

GPLv2 or later. See [LICENSE](LICENSE).
