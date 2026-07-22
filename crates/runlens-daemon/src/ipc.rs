use crate::state::DaemonState;
use runlens_core::event_v2::EventV2;
use runlens_core::protocol::{self, IpcMessage, JsonRpcError, JsonRpcResponse};
use serde_json::Value;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Start the IPC server on the given port. Runs until the shutdown
/// notify fires or the listener fails.
pub async fn serve(
    port: u16,
    state: Arc<RwLock<DaemonState>>,
    shutdown: Arc<Notify>,
) -> anyhow::Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    info!(addr = %addr, "daemon ipc listening");

    loop {
        tokio::select! {
            biased;
            _ = shutdown.notified() => {
                info!("ipc server shutting down");
                break;
            }
            accepted = listener.accept() => {
                let (stream, peer) = match accepted {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("accept error: {e}");
                        continue;
                    }
                };
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, &state).await {
                        warn!("connection from {peer}: {e}");
                    }
                });
            }
        }
    }
    Ok(())
}

async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    state: &Arc<RwLock<DaemonState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let n = buf_reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }

        let response = dispatch(&line, state).await;
        let mut raw = serde_json::to_vec(&response)?;
        raw.push(b'\n');
        writer.write_all(&raw).await?;
    }
    Ok(())
}

async fn dispatch(
    line: &str,
    state: &Arc<RwLock<DaemonState>>,
) -> serde_json::Value {
    let msg = match IpcMessage::parse(line.as_bytes()) {
        Ok(m) => m,
        Err(e) => return error_response(serde_json::Value::Null, e),
    };

    match msg {
        IpcMessage::Request(req) => {
            let id: serde_json::Value = match &req.id {
                runlens_core::protocol::JsonRpcId::Num(n) => serde_json::json!(n),
                runlens_core::protocol::JsonRpcId::Str(s) => serde_json::json!(s),
                runlens_core::protocol::JsonRpcId::Null => serde_json::Value::Null,
            };
            match handle_method(&req.method, req.params, state).await {
                Ok(result) => {
                    serde_json::to_value(JsonRpcResponse {
                        jsonrpc: protocol::JSON_RPC_VERSION.into(),
                        id: req.id,
                        result,
                    })
                    .unwrap_or_default()
                }
                Err(e) => error_response(id, e),
            }
        }
        IpcMessage::Notification(notif) => {
            match handle_method(&notif.method, notif.params, state).await {
                Ok(_) => serde_json::Value::Null,
                Err(e) => {
                    warn!("notification handler failed: {}: {e}", notif.method);
                    serde_json::Value::Null
                }
            }
        }
    }
}

fn error_response(id: serde_json::Value, err: JsonRpcError) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": err.code,
            "message": err.message,
        }
    })
}

async fn handle_method(
    method: &str,
    params: Option<Value>,
    state: &Arc<RwLock<DaemonState>>,
) -> Result<Value, JsonRpcError> {
    match method {
        protocol::methods::DAEMON_STATUS => {
            let s = state.read().await;
            Ok(protocol::responses::status(
                env!("CARGO_PKG_VERSION"),
                std::process::id() as u64,
                s.uptime_secs(),
                s.session_count().await,
                &s.db_path,
            ))
        }
        protocol::methods::DAEMON_SHUTDOWN => {
            info!("daemon.shutdown received");
            state.read().await.signal_shutdown();
            Ok(serde_json::json!({"shutdown": "ok"}))
        }
        protocol::methods::DAEMON_INGEST => {
            let payload = params
                .ok_or_else(|| JsonRpcError::invalid_params("missing params"))?;
            let event: EventV2 = serde_json::from_value(payload)
                .map_err(|e| JsonRpcError::invalid_params(format!("invalid event: {e}")))?;
            let s = state.read().await;
            s.ingest.ingest(event).await.map_err(|_| {
                JsonRpcError::internal_error("pipeline at capac