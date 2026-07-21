//! Bundle export + import roundtrip.
//!
//! Builds a minimal session/event graph in a fresh Repository, exports
//! it to a tarball, then re-imports into a second Repository and asserts
//! the imported events match.

#![allow(clippy::str_to_string)]

use runlens_core::chain;
use runlens_core::identifier::Identifier;
use runlens_core::model::{Event, EventSource, PrivacyClassification, ProjectInfo, Severity};
use runlens_bundle::{export_session, import_bundle, ExportOptions, ImportOptions};
use runlens_storage::{DiskArtifacts, Repository};
fn ev(seq: u64, kind: &str, session_id: &str, project_id: &str) -> Event {
    Event {
        event_id: Identifier::now().as_str().to_string(),
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        sequence: seq,
        source: EventSource::Core,
        kind: kind.into(),
        severity: Severity::Info,
        utc_timestamp: chrono::Utc::now(),
        monotonic_ns: seq * 1_000,
        duration_ns: None,
        correlation_id: None,
        parent_event_id: None,
        payload_version: 1,
        payload: serde_json::json!({"seq": seq}),
        classification: PrivacyClassification::Internal,
        previous_hash: None,
        current_hash: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn export_and_import_roundtrip_preserves_chain() {
    let dir_src = tempfile::tempdir().unwrap();
    let dir_dst = tempfile::tempdir().unwrap();
    let repo_src = Repository::open(dir_src.path().join("src.sqlite")).unwrap();
    let repo_dst = Repository::open(dir_dst.path().join("dst.sqlite")).unwrap();
    let artifacts = DiskArtifacts::open(dir_src.path().join("blobs")).unwrap();

    let project = ProjectInfo {
        project_id: Identifier::now().as_str().to_string(),
        name: "demo".into(),
        root: "~".into(),
        language_hints: vec!["rust".into()],
    };
    repo_src.ensure_project(&project).await.unwrap();

    let session_id = Identifier::now().as_str().to_string();
    let session = runlens_core::model::SessionInfo {
        session_id: session_id.clone(),
        project_id: project.project_id.clone(),
        state: runlens_core::model::SessionState::Complete,
        started_at: chrono::Utc::now(),
        stopped_at: Some(chrono::Utc::now()),
        command: Some("echo".into()),
        args: vec![],
        labels: vec![],
        source_event_count: 0,
        imported: false,
        bundle_origin: None,
    };
    repo_src.create_session(&session).await.unwrap();
    let mut prev = chain::GENESIS_HASH.to_string();
    for i in 0..10 {
        let mut e = ev(
            i,
            if i == 0 { "session.started" } else { "pty.stdout" },
            &session_id,
            &project.project_id,
        );
        let new_hash = chain::seal(&mut e, &prev);
        prev = new_hash;
        repo_src.append_event(&e).await.unwrap();
    }
    repo_src
        .update_session_state(&session_id, runlens_core::model::SessionState::Complete, Some(chrono::Utc::now()), Some(&prev), 10)
        .await
        .unwrap();

    let bundle_path = dir_src.path().join("demo.runlens");
    let _manifest = export_session(
        &repo_src,
        &session_id,
        &artifacts,
        ExportOptions {
            out_path: bundle_path.clone(),
            mask_root: Some("~".into()),
        },
    )
    .await
    .expect("export");

    let report = import_bundle(
        &repo_dst,
        &bundle_path,
        ImportOptions {
            extract_root: dir_dst.path().join("extract"),
            overwrite: false,
            redaction_allowlist: vec![],
        },
    )
    .await
    .expect("import");

    assert_eq!(report.events_imported, 10);
    let events_after = repo_dst.list_events(&session_id).await.unwrap();
    assert_eq!(events_after.len(), 10);
    chain::verify_chain(&events_after).expect("chain verification on import");
}
