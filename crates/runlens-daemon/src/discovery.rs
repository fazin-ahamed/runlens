use anyhow::{Context, Result};
use std::path::PathBuf;

const DAEMON_PORT_FILE: &str = ".runlens/daemon.port";

pub fn port_file_path(base: &std::path::Path) -> PathBuf {
    base.join(DAEMON_PORT_FILE)
}

pub async fn write_ports(base: &std::path::Path, tcp_port: u16, ws_port: u16) -> Result<PathBuf> {
    let path = port_file_path(base);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("creating .runlens/ for daemon port file")?;
    }
    let content = format!("{tcp_port}\n{ws_port}\n");
    tokio::fs::write(&path, &content)
        .await
        .context("writing daemon port file")?;
    Ok(path)
}

pub async fn write_port(base: &std::path::Path, port: u16) -> Result<PathBuf> {
    write_ports(base, port, 0).await
}

pub async fn remove_port(base: &std::path::Path) {
    let path = port_file_path(base);
    let _ = tokio::fs::remove_file(&path).await;
}

pub async fn read_port(base: &std::path::Path) -> Result<u16> {
    let path = port_file_path(base);
    let content = tokio::fs::read_to_string(&path)
        .await
        .context("reading daemon port file")?;
    let port: u16 = content
        .trim()
        .parse()
        .context("parsing daemon port")?;
    Ok(port)
}
