use std::io;
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command};
use std::time::Duration;

pub struct SshTunnel {
    child: Child,
    local_port: u16,
}

impl SshTunnel {
    pub fn new(username: Option<&str>, hostname: &str, remote_port: u16) -> io::Result<Self> {
        let local_port = pick_free_port()?;

        let host_arg = match username {
            Some(user) => format!("{user}@{hostname}"),
            None => hostname.to_string(),
        };

        let child = Command::new("ssh")
            .args([
                "-N",
                "-o", "StrictHostKeyChecking=accept-new",
                "-o", "ExitOnForwardFailure=yes",
                "-o", "ConnectTimeout=10",
                "-L", &format!("{local_port}:localhost:{remote_port}"),
                &host_arg,
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        Ok(Self { child, local_port })
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub fn is_ready(&self) -> bool {
        TcpStream::connect_timeout(
            &format!("127.0.0.1:{}", self.local_port).parse().unwrap(),
            Duration::from_millis(100),
        ).is_ok()
    }

    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn pick_free_port() -> io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}
