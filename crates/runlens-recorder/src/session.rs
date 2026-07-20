//! Session orchestration.
//!
//! A session is the top-level recording context: a single PTY child +
//! companion collectors (file watcher, git fingerprint, env fingerprint,
//! profiler).
//!
//! `Session::record` creates a project, opens a session row in the
//! `recording` state, spawns the user command in a PTY, and arms all
//! collectors. It blocks until the child exits and the row is sealed
//! in `complete` or `failed`.
//!
//! Privacy guarantees: every emitted payload goes through the
//! [`crate::redaction::Redactor`] before it is sealed into the chain and
//! persisted. Secret-bearing payloads from the child command are scanned
//! first, redacted second, then sent to storage.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use runlens_core::chain;
use runlens_core::identifier::Identifier;
use runlens_core::model::{
    Event, EventSource, PrivacyClassification, ProjectInfo, SessionInfo, SessionState, Severity,
};
use runlens_storage::Repository;
use tracing::{debug, warn};

use crate::dispatch::{Dispatcher, monotonic_now_ns};
use crate::env_fingerprint::capture_env_fingerprint;
use crate::git::capture_git_fingerprint;
use crate::profiler::Profiler;
use crate::pty::{run_pty, PtyOutcome, TestSummary};

/// User-facing options knob.
#[derive(Debug, Clone)]
pub struct RecordingOptions {
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
    pub env: indexmap::IndexMap<String, String>,
    pub label: Option<String>,
    pub watch_paths: Vec<PathBuf>,
    pub enable_profiler: bool,
    pub enable_git: bool,
    pub enable_env: bool,
    pub profiler_interval_ms: u64,
    pub test_adapter: Option<TestAdapterKind>,
    pub fail_on_findings: bool,
}

/// What test-adapter to attach to the PTY output stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestAdapterKind {
    Junit,
    Pytest,
    Vitest,
    Gotest,
}

/// Final shape of what a session produced, returned for tests/CLI use.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub project_id: String,
    pub state: SessionState,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub stopped_at: Option<chrono::DateTime<chrono::Utc>>,
    pub exit_code: Option<i32>,
    pub source_event_count: u64,
    pub redaction_findings_total: u64,
    pub git_available: bool,
    pub tests: TestSummary,
}

impl SessionSummary {
    pub fn is_success(&self) -> bool {
        matches!(self.state, SessionState::Complete)
    }
}

pub type SessionHandle = SessionSummary;
pub use SessionSummary as SessionResult;

pub struct Session;

impl Session {
    /// Run a full recording session to completion.
    pub async fn record(repo: Repository, opts: RecordingOptions) -> Result<SessionSummary> {
        if opts.command.is_empty() {
            return Err(anyhow!("recording requires a command to spawn"));
        }
        let project_info = upsert_project(&repo, &opts.working_dir).await?;
        debug!(project_id = %project_info.project_id, "project ensured");

        let session_id = Identifier::now().as_str().to_string();
        let start_ts = Utc::now();
        let mut labels_vec: Vec<String> = Vec::new();
        if let Some(l) = &opts.label {
            labels_vec.push(l.clone());
        }
        let session_info = SessionInfo {
            session_id: session_id.clone(),
            project_id: project_info.project_id.clone(),
            state: SessionState::Recording,
            started_at: start_ts,
            stopped_at: None,
            command: if opts.command.is_empty() {
                None
            } else {
                Some(opts.command.clone())
            },
            args: opts.args.clone(),
            labels: labels_vec,
            source_event_count: 0,
            imported: false,
            bundle_origin: None,
        };
        repo.create_session(&session_info)
            .await
            .context("create_session")?;
        repo.update_session_state(
            &session_id,
            SessionState::Recording,
            None,
            None,
            0,
        )
        .await?;

        let dispatcher = Dispatcher::new(
            repo.clone(),
            project_info.project_id.clone(),
            session_id.clone(),
            chain::GENESIS_HASH.to_string(),
        );

        // Genesis event so the hash chain is non-empty.
        emit_core(
            &dispatcher,
            "session.started",
            Severity::Info,
            serde_json::json!({
                "session_id": session_id,
                "project_id": project_info.project_id,
                "command": opts.command,
                "args": opts.args,
                "working_dir": opts.working_dir.to_string_lossy(),
                "label": opts.label,
            }),
        )
        .await?;

        let git_available = if opts.enable_git {
            match capture_git_fingerprint(&opts.working_dir).await {
                Ok(git) => {
                    emit_core(
                        &dispatcher,
                        "git.snapshot",
                        Severity::Info,
                        serde_json::to_value(&git).unwrap_or_default(),
                    )
                    .await
                    .ok();
                    true
                }
                Err(_) => false,
            }
        } else {
            false
        };

        if opts.enable_env {
            let env_fp = capture_env_fingerprint(&opts.env);
            emit_core(
                &dispatcher,
                "env.snapshot",
                Severity::Info,
                serde_json::to_value(&env_fp).unwrap_or_default(),
            )
            .await
            .ok();
        }

        let profiler = if opts.enable_profiler {
            let interval = Duration::from_millis(opts.profiler_interval_ms.max(50));
            Some(Profiler::start(interval, dispatcher.clone()))
        } else {
            None
        };

        let pty_outcome: PtyOutcome = run_pty(
            &opts.command,
            &opts.args,
            &opts.env,
            &opts.working_dir,
            dispatcher.clone(),
            opts.test_adapter,
        )
        .await
        .context("pty run")?;

        if let Some(p) = profiler {
            p.stop().await;
        }

        let final_state = if pty_outcome.exit_status.success() {
            SessionState::Complete
        } else {
            SessionState::Failed
        };
        let events_vec = repo.list_events(&session_id).await.unwrap_or_default();
        let final_count = events_vec.len() as u64;
        let redaction_total: u64 = events_vec
            .iter()
            .filter(|e| e.classification == PrivacyClassification::Sensitive || e.classification == PrivacyClassification::Confidential)
            .count() as u64;

        repo.update_session_state(
            &session_id,
            final_state.clone(),
            Some(Utc::now()),
            None,
            final_count,
        )
        .await?;

        emit_core(
            &dispatcher,
            "session.stopped",
            Severity::Info,
            serde_json::json!({
                "exit_code": pty_outcome.exit_status.code,
                "success": pty_outcome.exit_status.success(),
                "wall_ms": pty_outcome.wall_clock_ms,
                "test_summary": pty_outcome.test_summary,
            }),
        )
        .await
        .ok();

        if opts.fail_on_findings && redaction_total > 0 {
            warn!(
                findings = redaction_total,
                "session would fail by user policy: redaction findings non-zero"
            );
        }

        Ok(SessionSummary {
            session_id,
            project_id: project_info.project_id,
            state: final_state,
            started_at: start_ts,
            stopped_at: Some(Utc::now()),
            exit_code: pty_outcome.exit_status.code,
            source_event_count: final_count,
            redaction_findings_total: redaction_total,
            git_available,
            tests: pty_outcome.test_summary,
        })
    }
}

async fn upsert_project(repo: &Repository, root: &Path) -> Result<ProjectInfo> {
    let canonical = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf());
    let canonical_str = canonical.to_string_lossy().into_owned();

    let recent = repo.list_recent_sessions(40).await.unwrap_or_default();
    for sess in recent {
        if let Ok(Some(p)) = repo.get_project(&sess.project_id).await {
            if p.root == canonical_str {
                return Ok(p);
            }
        }
    }
    let project_id = Identifier::now().as_str().to_string();
    let name = canonical
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "runlens-project".into());
    let project = ProjectInfo {
        project_id: project_id.clone(),
        name,
        root: canonical_str.clone(),
        language_hints: vec![],
    };
    repo.ensure_project(&project).await?;
    Ok(repo
        .get_project(&project_id)
        .await?
        .unwrap_or(project))
}

async fn emit_core(
    dispatcher: &Dispatcher,
    kind: &str,
    severity: Severity,
    payload: serde_json::Value,
) -> Result<()> {
    let now = Utc::now();
    let event = Event {
        event_id: String::new(),
        session_id: dispatcher.session_id().to_string(),
        project_id: dispatcher.project_id().to_string(),
        sequence: 0,
        source: EventSource::Core,
        kind: kind.to_string(),
        severity,
        utc_timestamp: now,
        monotonic_ns: monotonic_now_ns(),
        duration_ns: None,
        correlation_id: None,
        parent_event_id: None,
        payload_version: 1,
        payload,
        classification: PrivacyClassification::Internal,
        previous_hash: None,
        current_hash: None,
    };
    dispatcher.emit(event).await?;
    Ok(())
}
