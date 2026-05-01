pub mod serial;
pub mod ssh_tunnel;
pub mod vnc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsoleStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}
