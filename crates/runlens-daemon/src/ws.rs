use crate::pipeline::IngestHandle;
use crate::subscription::SubscriptionManager;
use futures_util::{SinkExt, StreamExt};
use runlens_core::event_v2::EventV2;
use runlens_core::protocol::{self, IpcMessage, JsonRpcError};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Notify};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

pub async fn serve(
    port: u16,
    ingest: IngestHandle,
    subscriptions: Arc<SubscriptionManager>,
    shutdown: Arc<Notify>,
) -> anyhow::Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    info!(addr = %addr, "daemon ws listening");

    loop {
        tokio::select! {
            biased;
            _ = shutdown.notified() => {
                info!("ws server shutting down");
                break;
            }
            accepted = listener.accept() => {
                let (stream, peer) = match accepted {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("ws accept error: {e}");
                        continue;
                    }
                };
                let ingest = ingest.clone();
                let subs = subscriptions.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, ingest, subs).await {
                        warn!("ws connection from {peer}: {e}");
                    }
                });
            }
        }
    }
    Ok(())
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    ingest: IngestHandle,
    subscriptions: Arc<SubscriptionManager>,
) -> Result<(), anyhow::Error> {
    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let (_sub_id, mut sub_rx) = subscriptions.subscribe(None).await;

    loop {
        tokio::select! {
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_message(&text, &ingest).await {
                            warn!("ws message error: {e}");
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = ws_sender.send(Message::Pong(payload)).await;
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Binary(_))) => {}
                    Some(Ok(Message::Frame(_))) => {}
                    Some(Err(e)) => {
                        warn!("ws protocol error: {e}");
                        break;
                    }
                }
            }
            event = sub_rx.recv() => {
                match event {
                    Ok(ev) => {
                        let notification = serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": "event.ingested",
                            "params": ev,
                        });
                        let text = serde_json::to_string(&notification)?;
                        if ws_sender.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("ws sub lagged by {n}");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    Ok(())
}

async fn handle_message(text: &str, ingest: &IngestHandle) -> Result<(), JsonRpcError> {
    let msg = IpcMessage::parse(text.as_bytes())?;
    match msg {
        IpcMessage::Notification(notif) => match notif.method.as_str() {
            protocol::methods::EVENT_EMIT | protocol::methods::DAEMON_INGEST => {
                let event: EventV2 = serde_json::from_value(notif.params.unwrap_or_default())
                    .map_err(|e| JsonRpcError::invalid_params(format!("invalid event payload: {e}")))?;
                ingest
                    .ingest(event)
                    .await
                    .map_err(|_| JsonRpcError::internal_error("ingest channel closed"))?;
            },
            _ => {
                warn!("unknown ws notification method: {}", notif.method);
            },
        },
        IpcMessage::Request(req) => {
            warn!("unexpected request in ws message: {}", req.method);
        },
    }
    Ok(())
}
