#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

#![allow(clippy::doc_markdown)]

use runlens_daemon::{discovery, ipc, pipeline, state::DaemonState, subscription::SubscriptionManager, ws};
use runlens_storage::repo::Repository;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

fn find_project_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let mut dir = Some(cwd.as_path());
    while let Some(d) = dir {
        if d.join(".runlens").exists() || d.join(".git").exists() {
            return Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    None
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,runlens_daemon=info".into()),
        )
        .init();

    let project_root = find_project_root()
        .ok_or_else(|| anyhow::anyhow!("no .runlens or .git found in path"))?;
    let db_path = project_root.join(".runlens").join("runlens.sqlite");

    let port: u16 = std::env::var("RUNLENS_DAEMON_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9876);

    let ws_port: u16 = std::env::var("RUNLENS_WS_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(6790);

    let port_path = discovery::write_ports(&project_root, port, ws_port).await?;
    info!(port, ws_port, path = %port_path.display(), "daemon ports written");

    let repo = Repository::open(&db_path)?;
    let subscriptions = Arc::new(SubscriptionManager::new());
    let ingest = pipeline::start_ingest_worker(repo.clone(), subscriptions.clone());

    let shutdown = Arc::new(tokio::sync::Notify::new());
    let state = Arc::new(RwLock::new(DaemonState::new(
        db_path.to_string_lossy().to_string(),
        ingest,
        subscriptions,
        repo,
    )));

    let daemon_shutdown = shutdown.clone();
    let ipc_fut = ipc::serve(port, state.clone(), daemon_shutdown);

    let ws_ingest = state.read().await.ingest.clone();
    let ws_subs = state.read().await.subscriptions.clone();
    let ws_shutdown = shutdown.clone();
    let ws_fut = ws::serve(ws_port, ws_ingest, ws_subs, ws_shutdown);

    #[cfg(unix)]
    let sig_fut = {
        use tokio::signal::unix;
        let mut sigterm = unix::signal(unix::SignalKind::terminate())?;
        let mut sigint = unix::signal(unix::SignalKind::interrupt())?;
        async move {
            tokio::select! {
                _ = sigterm.recv() => info!("received SIGTERM"),
                _ = sigint.recv() => info!("received SIGINT"),
            }
        }
    };

    #[cfg(not(unix))]
    let sig_fut = async {
        let _ = tokio::signal::ctrl_c().await;
        info!("received CTRL+C");
    };

    tokio::select! {
        ipc_outcome = ipc_fut => {
            if let Err(e) = ipc_outcome {
                info!("ipc server exited: {e}");
            }
        }
        ws_outcome = ws_fut => {
            if let Err(e) = ws_outcome {
                info!("ws server exited: {e}");
            }
        }
        _ = sig_fut => {
            shutdown.notify_one();
        }
    }

    discovery::remove_port(&project_root).await;
    info!("daemon stopped");
    Ok(())
}
