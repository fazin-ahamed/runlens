use runlens_core::event_v2::EventV2;
use runlens_core::identifier::Identifier;
use runlens_core::model::{EventSource, PrivacyClassification, Severity};
use runlens_daemon::pipeline;
use runlens_daemon::subscription::SubscriptionManager;
use runlens_storage::repo::Repository;
use std::sync::Arc;
use tokio::time::{timeout, Duration};

fn make_event(session_id: &str, project_id: &str, seq: u64) -> EventV2 {
    EventV2::new(
        Identifier::now(),
        Identifier::from_string(session_id).unwrap(),
        Identifier::from_string(project_id).unwrap(),
        seq,
        EventSource::Cli,
        "test.integration",
        Severity::Info,
        serde_json::json!({"seq": seq}),
        PrivacyClassification::Internal,
    )
}

#[tokio::test]
async fn pipeline_persists_events() {
    let repo = Repository::in_memory().unwrap();
    let subscriptions = Arc::new(SubscriptionManager::new());
    let handle = pipeline::start_ingest_worker(repo.clone(), subscriptions);

    let sid = Identifier::now().to_string();
    let pid = Identifier::now().to_string();
    let pid2 = pid.clone();

    let project = runlens_core::model::ProjectInfo {
        project_id: pid.clone(),
        name: "integration-test".into(),
        root: "/tmp".into(),
        language_hints: vec![],
    };
    repo.ensure_project(&project).unwrap();
    let session = runlens_core::model::SessionInfo {
        session_id: sid.clone(),
        project_id: pid,
        state: runlens_core::model::SessionState::Recording,
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

    for i in 0..5 {
        handle.ingest(make_event(&sid, &pid2, i)).await.unwrap();
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    let events = repo.list_events(&sid).unwrap();
    assert_eq!(events.len(), 5, "pipeline should persist 5 events");

    let sess = repo.get_session(&sid).unwrap();
    assert_eq!(sess.source_event_count, 5);
}

#[tokio::test]
async fn pipeline_fans_out_to_subscribers() {
    let repo = Repository::in_memory().unwrap();
    let subscriptions = Arc::new(SubscriptionManager::new());
    let handle = pipeline::start_ingest_worker(repo.clone(), subscriptions.clone());

    let sid = Identifier::now().to_string();
    let pid = Identifier::now().to_string();
    let pid2 = pid.clone();

    let project = runlens_core::model::ProjectInfo {
        project_id: pid.clone(),
        name: "fanout-test".into(),
        root: "/tmp".into(),
        language_hints: vec![],
    };
    repo.ensure_project(&project).unwrap();
    let session = runlens_core::model::SessionInfo {
        session_id: sid.clone(),
        project_id: pid,
        state: runlens_core::model::SessionState::Recording,
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

    let (_sub_id, mut rx) = subscriptions.subscribe(Some(sid.clone())).await;

    handle.ingest(make_event(&sid, &pid2, 0)).await.unwrap();

    let got = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for event")
        .expect("channel closed");
    assert_eq!(got.sequence, 0);
}
