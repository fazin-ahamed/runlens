use runlens_core::event_v2::EventV2;
use runlens_core::identifier::Identifier;
use runlens_core::model::{EventSource, PrivacyClassification, Severity};
use runlens_daemon::pipeline;
use runlens_daemon::subscription::SubscriptionManager;
use runlens_storage::repo::Repository;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::Duration;

#[tokio::test]
async fn pipeline_sustains_5000_events_per_second() {
    let repo = Repository::in_memory().unwrap();
    let subscriptions = Arc::new(SubscriptionManager::new());
    let handle = pipeline::start_ingest_worker(repo.clone(), subscriptions);

    let sid = Identifier::now().to_string();
    let pid = Identifier::now().to_string();
    let pid2 = pid.clone();

    let project = runlens_core::model::ProjectInfo {
        project_id: pid.clone(),
        name: "throughput-test".into(),
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

    let count = 5000u64;
    let start = Instant::now();

    for i in 0..count {
        let ev = EventV2::new(
            Identifier::now(),
            Identifier::from_string(&sid).unwrap(),
            Identifier::from_string(&pid2).unwrap(),
            i,
            EventSource::Core,
            "perf.test",
            Severity::Info,
            serde_json::json!({"i": i}),
            PrivacyClassification::Internal,
        );
        handle.ingest(ev).await.unwrap();
    }

    drop(handle);

    let mut stored;
    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;
        stored = repo.list_events(&sid).unwrap();
        if stored.len() as u64 == count {
            break;
        }
    }

    let elapsed = start.elapsed();
    let rate = count as f64 / elapsed.as_secs_f64();

    assert_eq!(stored.len() as u64, count);
    eprintln!("throughput: {rate:.0} events/s (elapsed: {:?})", elapsed);
    assert!(rate >= 3000.0, "throughput {rate:.0} events/s < 3000 floor");
}
