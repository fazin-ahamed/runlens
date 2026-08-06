use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Debug, Clone)]
pub enum ProxyMode {
    Record,
    Replay,
    Mixed,
}

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub port: u16,
    pub mode: ProxyMode,
    pub bind_addr: String,
    pub tls: bool,
    pub session_id: Option<String>,
}

pub struct Proxy;

impl Proxy {
    pub fn new(_config: ProxyConfig, _session_id: Option<String>) -> Self {
        Self
    }

    pub async fn serve(&self, _shutdown: Arc<Notify>) -> anyhow::Result<()> {
        Ok(())
    }
}
