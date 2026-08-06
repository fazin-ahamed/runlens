use crate::state::DaemonState;
use runlens_core::event_v2::EventV2;
use runlens_core::model::{SessionInfo, SessionState};
use runlens_core::protocol::{self, IpcMessage, JsonRpcError, JsonRpcId, JsonRpcResponse};
use runlens_graph::critical::critical_path;
use runlens_graph::diff;
use runlens_graph::graph::GraphBuilder;
use runlens_graph::span::follow_chain;
use runlens_query::{run_explain, run_query};
use serde_json::Value;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub async fn serve(port: u16, state: Arc<RwLock<DaemonState>>, shutdown: Arc<Notify>) -> anyhow::Result<()> {
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

async fn dispatch(line: &str, state: &Arc<RwLock<DaemonState>>) -> serde_json::Value {
    let msg = match IpcMessage::parse(line.as_bytes()) {
        Ok(m) => m,
        Err(e) => return error_response(serde_json::Value::Null, e),
    };

    match msg {
        IpcMessage::Request(req) => {
            let id: serde_json::Value = match &req.id {
                JsonRpcId::Num(n) => serde_json::json!(n),
                JsonRpcId::Str(s) => serde_json::json!(s),
                JsonRpcId::Null => serde_json::Value::Null,
            };
            match handle_method(&req.method, req.params, state).await {
                Ok(result) => serde_json::to_value(JsonRpcResponse {
                    jsonrpc: protocol::JSON_RPC_VERSION.into(),
                    id: req.id,
                    result,
                })
                .unwrap_or_default(),
                Err(e) => error_response(id, e),
            }
        },
        IpcMessage::Notification(notif) => match handle_method(&notif.method, notif.params, state).await {
            Ok(_) => serde_json::Value::Null,
            Err(e) => {
                warn!("notification handler failed: {}: {e}", notif.method);
                serde_json::Value::Null
            },
        },
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
        },
        protocol::methods::DAEMON_SHUTDOWN => {
            info!("daemon.shutdown received");
            state.read().await.signal_shutdown();
            Ok(serde_json::json!({"shutdown": "ok"}))
        },
        protocol::methods::DAEMON_INGEST => {
            let payload = params.ok_or_else(|| JsonRpcError::invalid_params("missing event payload"))?;
            let event: EventV2 = serde_json::from_value(payload)
                .map_err(|e| JsonRpcError::invalid_params(format!("invalid event: {e}")))?;
            let s = state.read().await;
            s.ingest
                .ingest(event)
                .await
                .map_err(|_| JsonRpcError::internal_error("ingest channel closed; daemon shutting down"))?;
            Ok(serde_json::json!({"ingested": true}))
        },
        protocol::methods::DAEMON_SUBSCRIBE => {
            let session_id: Option<String> =
                params.and_then(|v| v.get("session_id").and_then(|s| s.as_str().map(String::from)));
            let s = state.read().await;
            let (sub_id, _rx) = s.subscriptions.subscribe(session_id).await;
            Ok(serde_json::json!({"subscription_id": sub_id.0}))
        },
        protocol::methods::SESSION_LIST => {
            let s = state.read().await;
            let sessions = s
                .repo
                .list_recent_sessions(100)
                .map_err(|e| JsonRpcError::internal_error(format!("listing sessions: {e}")))?;
            Ok(serde_json::to_value(sessions)
                .map_err(|e| JsonRpcError::internal_error(format!("serializing sessions: {e}")))?)
        },
        protocol::methods::SESSION_GET => {
            let session_id = params
                .and_then(|v| v.get("id").and_then(|s| s.as_str().map(String::from)))
                .ok_or_else(|| JsonRpcError::invalid_params("missing session id"))?;
            let s = state.read().await;
            let session = s
                .repo
                .get_session(&session_id)
                .map_err(|e| JsonRpcError::internal_error(format!("session {session_id}: {e}")))?;
            let events = s
                .repo
                .list_events(&session_id)
                .map_err(|e| JsonRpcError::internal_error(format!("events for {session_id}: {e}")))?;
            let mut session_json = serde_json::to_value(&session)
                .map_err(|e| JsonRpcError::internal_error(format!("serializing session: {e}")))?;
            if let Some(obj) = session_json.as_object_mut() {
                obj.insert(
                    "events".into(),
                    serde_json::to_value(&events)
                        .map_err(|e| JsonRpcError::internal_error(format!("serializing events: {e}")))?,
                );
            }
            Ok(session_json)
        },
        protocol::methods::RECORD_START => {
            let s = state.read().await;
            let session_id = format!("rec-{}", DaemonState::next_session_num());
            let info = SessionInfo {
                session_id: session_id.clone(),
                project_id: "default".into(),
                state: SessionState::Recording,
                started_at: chrono::Utc::now(),
                stopped_at: None,
                command: params.and_then(|v| v.get("command").and_then(|c| c.as_str().map(String::from))),
                args: vec![],
                labels: vec![],
                source_event_count: 0,
                imported: false,
                bundle_origin: None,
            };
            s.repo
                .create_session(&info)
                .map_err(|e| JsonRpcError::internal_error(format!("creating session: {e}")))?;
            s.register_session(session_id.clone()).await;
            Ok(serde_json::json!({"session_id": session_id}))
        },
        protocol::methods::RECORD_STOP => {
            let s = state.read().await;
            let session_id = params.and_then(|v| v.get("session_id").and_then(|s| s.as_str().map(String::from)));
            if let Some(sid) = session_id {
                s.repo
                    .update_session_state(&sid, SessionState::Complete, Some(chrono::Utc::now()), None, 0)
                    .map_err(|e| JsonRpcError::internal_error(format!("stopping session {sid}: {e}")))?;
                s.unregister_session(&sid).await;
            }
            Ok(serde_json::json!({"stopped": true}))
        },
        protocol::methods::GRAPH_TRACE => {
            let trace_id = params
                .and_then(|v| v.get("trace_id").and_then(|s| s.as_str().map(String::from)))
                .ok_or_else(|| JsonRpcError::invalid_params("missing trace_id"))?;
            let s = state.read().await;
            let graph = GraphBuilder::new(&s.repo)
                .load(&trace_id)
                .map_err(|e| JsonRpcError::internal_error(format!("graph error: {e}")))?;
            serde_json::to_value(&graph).map_err(|e| JsonRpcError::internal_error(format!("serialize: {e}")))
        },
        protocol::methods::GRAPH_CRITICAL => {
            let trace_id = params
                .and_then(|v| v.get("trace_id").and_then(|s| s.as_str().map(String::from)))
                .ok_or_else(|| JsonRpcError::invalid_params("missing trace_id"))?;
            let s = state.read().await;
            let graph = GraphBuilder::new(&s.repo)
                .load(&trace_id)
                .map_err(|e| JsonRpcError::internal_error(format!("graph error: {e}")))?;
            let path = critical_path(&graph);
            serde_json::to_value(&path).map_err(|e| JsonRpcError::internal_error(format!("serialize: {e}")))
        },
        protocol::methods::GRAPH_COMPARE => {
            let p = params.ok_or_else(|| JsonRpcError::invalid_params("missing params"))?;
            let a = p
                .get("a")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError::invalid_params("missing a"))?;
            let b = p
                .get("b")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError::invalid_params("missing b"))?;
            let s = state.read().await;
            let repo = &s.repo;
            let ga = GraphBuilder::new(repo)
                .load_session(a)
                .map_err(|e| JsonRpcError::internal_error(format!("graph error: {e}")))?;
            let gb = GraphBuilder::new(repo)
                .load_session(b)
                .map_err(|e| JsonRpcError::internal_error(format!("graph error: {e}")))?;
            let cmp = diff::compare(&ga, &gb);
            serde_json::to_value(&cmp).map_err(|e| JsonRpcError::internal_error(format!("serialize: {e}")))
        },
        protocol::methods::GRAPH_CHAIN => {
            let p = params.ok_or_else(|| JsonRpcError::invalid_params("missing params"))?;
            let trace_id = p
                .get("trace_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError::invalid_params("missing trace_id"))?;
            let span_id = p
                .get("span_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError::invalid_params("missing span_id"))?;
            let s = state.read().await;
            let graph = GraphBuilder::new(&s.repo)
                .load(trace_id)
                .map_err(|e| JsonRpcError::internal_error(format!("graph error: {e}")))?;
            let chain = follow_chain(&graph, span_id);
            serde_json::to_value(&chain).map_err(|e| JsonRpcError::internal_error(format!("serialize: {e}")))
        },
        protocol::methods::QUERY_EXECUTE => {
            let rql = params
                .and_then(|v| v.get("rql").and_then(|s| s.as_str().map(String::from)))
                .ok_or_else(|| JsonRpcError::invalid_params("missing rql"))?;
            let s = state.read().await;
            let conn_guard = s.repo.conn().lock().unwrap();
            let rows =
                run_query(&conn_guard, &rql).map_err(|e| JsonRpcError::internal_error(format!("query error: {e}")))?;
            Ok(serde_json::json!({"rows": rows}))
        },
        protocol::methods::QUERY_EXPLAIN => {
            let rql = params
                .and_then(|v| v.get("rql").and_then(|s| s.as_str().map(String::from)))
                .ok_or_else(|| JsonRpcError::invalid_params("missing rql"))?;
            let s = state.read().await;
            let conn_guard = s.repo.conn().lock().unwrap();
            let rows = run_explain(&conn_guard, &rql)
                .map_err(|e| JsonRpcError::internal_error(format!("explain error: {e}")))?;
            Ok(serde_json::json!({"rows": rows}))
        },
        _ => Err(JsonRpcError::method_not_found(format!("unknown method: {method}"))),
    }
}
