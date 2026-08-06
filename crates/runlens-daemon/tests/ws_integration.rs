#![allow(clippy::useless_format)]

use futures_util::{SinkExt, StreamExt};
use runlens_core::event_v2::EventV2;
use runlens_core::identifier::Identifier;
use runlens_core::model::{EventSource, PrivacyClassification, ProjectInfo, SessionInfo, SessionState, Severity};
use runlens_daemon::{pipeline, subscription::SubscriptionManager, ws};
use runlens_storage::repo::Repository;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

fn make_event(session_id: &str, project_id: &str, seq: u64) -> EventV2 {
    EventV2::new(
        Identifier::now(),
        Identifier::from_string(session_id).unwrap(),
        Identifier::from_string(project_id).unwrap(),
        seq,
        EventSource::Browser,
        "test.ws_integration",
        Severity::Info,
        serde_json::json!({"seq": seq}),
        PrivacyClassification::Internal,
    )
}

async fn setup(port: u16) -> (Repository, String, String, Arc<Notify>) {
    let repo = Repository::in_memory().unwrap();
    let subscriptions = Arc::new(SubscriptionManager::new());
    let handle = pipeline::start_ingest_worker(repo.clone(), subscriptions.clone());

    let sid = Identifier::now().to_string();
    let pid = Identifier::now().to_string();

    let project = ProjectInfo {
        project_id: pid.clone(),
        name: "ws-integration-test".into(),
        root: "/tmp".into(),
        language_hints: vec![],
    };
    repo.ensure_project(&project).unwrap();

    let session = SessionInfo {
        session_id: sid.clone(),
        project_id: pid.clone(),
        state: SessionState::Recording,
        started_at: chrono::Utc::now(),
        stopped_at: None,
        command: None,
        args: vec![],
        labels: vec![],
        source_event_count: 0,
        imported: false,
        bundle_origin: None,
    };
    repo.create_session(&session).unwrap();

    let shutdown = Arc::new(Notify::new());

    let ws_shutdown = shutdown.clone();
    tokio::spawn(async move {
        let _ = ws::serve(port, handle, subscriptions, ws_shutdown).await;
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    (repo, sid, pid, shutdown)
}

#[tokio::test]
async fn ws_ingest_persists_event() {
    let (repo, sid, pid, shutdown) = setup(19790).await;

    let url = format!("ws://127.0.0.1:19790");
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, _read) = ws_stream.split();

    let event = make_event(&sid, &pid, 0);
    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "daemon.ingest",
        "params": event,
    });
    let text = serde_json::to_string(&notification).unwrap();
    write.send(Message::Text(text)).await.unwrap();

    tokio::time::sleep(Duration::from_millis(300)).await;

    let events = repo.list_events(&sid).unwrap();
    assert!(!events.is_empty(), "should have persisted at least one event");
    assert_eq!(events[0].event_id, event.event_id);

    shutdown.notify_one();
    tokio::time::sleep(Duration::from_millis(50)).await;
}

#[tokio::test]
async fn ws_subscription_forwards_events() {
    let (_repo, sid, pid, shutdown) = setup(19791).await;

    let url = format!("ws://127.0.0.1:19791");
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let event = make_event(&sid, &pid, 0);
    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "daemon.ingest",
        "params": event,
    });
    let text = serde_json::to_string(&notification).unwrap();
    write.send(Message::Text(text)).await.unwrap();

    let message = timeout(Duration::from_secs(2), read.next())
        .await
        .expect("timed out waiting for forwarded event")
        .expect("stream ended")
        .expect("ws error");

    match message {
        Message::Text(t) => {
            let v: serde_json::Value = serde_json::from_str(&t).unwrap();
            assert_eq!(v["jsonrpc"], "2.0");
            assert_eq!(v["method"], "event.ingested");
            assert!(v["params"].is_object());
        },
        other => panic!("expected Text message, got {:?}", other),
    }

    shutdown.notify_one();
    tokio::time::sleep(Duration::from_millis(50)).await;
}
