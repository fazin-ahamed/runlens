use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
pub struct ProxyArgs {
    #[command(subcommand)]
    pub action: ProxyAction,
}

#[derive(Debug, Subcommand)]
pub enum ProxyAction {
    Start {
        #[arg(long, default_value = "8080")]
        port: u16,
        #[arg(long, default_value = "record")]
        mode: String,
        #[arg(long)]
        daemon: Option<String>,
    },
    InstallCa,
    RemoveCa,
}

pub async fn run(args: &ProxyArgs, _workspace: &crate::paths::WorkspacePaths) -> anyhow::Result<()> {
    match &args.action {
        ProxyAction::Start { port, mode, .. } => {
            let config = runlens_proxy::proxy::ProxyConfig {
                port: *port,
                mode: match mode.as_str() {
                    "replay" => runlens_proxy::proxy::ProxyMode::Replay,
                    "mixed" => runlens_proxy::proxy::ProxyMode::Mixed,
                    _ => runlens_proxy::proxy::ProxyMode::Record,
                },
                bind_addr: "127.0.0.1".into(),
                tls: false,
                session_id: None,
            };
            let proxy = runlens_proxy::proxy::Proxy::new(config, None);
            let shutdown = std::sync::Arc::new(tokio::sync::Notify::new());
            let s = shutdown.clone();
            tokio::spawn(async move {
                tokio::signal::ctrl_c().await.ok();
                s.notify_one();
            });
            proxy.serve(shutdown).await?;
        },
        ProxyAction::InstallCa => {
            let store = runlens_proxy::tls::CaStore::load_or_generate(std::path::Path::new("."))?;
            store.install()?;
            println!("CA certificate installed. You may need to restart your browser.");
        },
        ProxyAction::RemoveCa => {
            let store = runlens_proxy::tls::CaStore::load_or_generate(std::path::Path::new("."))?;
            store.remove()?;
            println!("CA certificate removed.");
        },
    }
    Ok(())
}
