use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use runlens_core::chain;
use runlens_core::identifier::Identifier;
use runlens_core::model::{
    Event, EventSource, PrivacyClassification, ProjectInfo, SessionInfo, SessionState, Severity,
};
use runlens_storage::Repository;
use tracing::{debug, warn};

use crate::dispatch::{monotonic_now_ns, Dispatcher};
use crate::env_fingerprint::capture_env_fingerprint;
use crate::file_watcher::{default_ignore, FsWatcher};
use crate::git::capture_git_fingerprint;
use crate::profiler::Profiler;
use crate::pty;
use crate::pty::TestSummary;

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
    pub test_adapter_hint: Option<TestAdapterKind>,
    pub fail_on_findings: bool,
    pub max_events: Option<u64>,
}

impl Default for RecordingOptions {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: vec![],
            working_dir: PathBuf::from("."),
            env: indexmap::IndexMap::new(),
            label: None,
            watch_paths: vec![],
            enable_profiler: true,
            enable_git: true,
            enable_env: true,
            profiler_interval_ms: 5000,
            test_adapter_hint: None,
            fail_on_findings: false,
            max_events: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestAdapterKind {
    Junit,
    Pytest,
    Vitest,
    Gotest,
}

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub session_id: String,
    pub project_id: String,
    pub state: SessionState,
    pub started_at: chrono::DateTime<Utc>,
    pub stopped_at: Option<chrono::DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub source_event_count: u64,
    pub redaction_findings_total: u64,
    pub git_available: bool,
    pub tests: TestSummary,
}

impl SessionSummary {
    pub fn is_success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

pub struct Session {
    repo: Repository,
    opts: RecordingOptions,
}

impl Session {
    pub fn new(repo: Repository, opts: RecordingOptions) -> Self {
        Self { repo, opts }
    }

    pub async fn record(&self) -> Result<SessionSummary> {
        let opts = &self.opts;
        let start_ts = Utc::now();
        let session_id = Identifier::now().as_str().to_string();
        let project_info = upsert_project(&self.repo, &opts.working_dir)?;
        let mut git_available = false;

        let labels_vec = opts.label.as_ref().map(|l| vec![l.clone()]).unwrap_or_default();

        let session_info = SessionInfo {
            session_id: session_id.clone(),
            project_id: project_info.project_id.clone(),
            state: SessionState::Preparing,
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
        self.repo
            .create_session(&session_info)
            .context("could not create session row in store")?;
        self.repo
            .update_session_state(&session_id, SessionState::Recording, None, None, 0)?;

        let dispatcher = Dispatcher::new(
            self.repo.clone(),
            project_info.project_id.clone(),
            session_id.clone(),
            chain::GENESIS_HASH.to_string(),
            opts.max_events,
        );

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
        )?;

        if opts.enable_git {
            match capture_git_fingerprint(&opts.working_dir).await {
                Ok(git) => {
                    git_available = true;
                    emit_core(
                        &dispatcher,
                        "git.snapshot",
                        Severity::Info,
                        serde_json::to_value(&git).unwrap_or_default(),
                    )?;
                },
                Err(e) => {
                    debug!(error=%e, "git fingerprint unavailable");
                },
            }
        }

        if opts.enable_env {
            let env_fp = capture_env_fingerprint(&opts.env);
            emit_core(
                &dispatcher,
                "env.fingerprint",
                Severity::Info,
                serde_json::to_value(&env_fp).unwrap_or_default(),
            )?;
        }

        let watch_roots = if opts.watch_paths.is_empty() {
            vec![opts.working_dir.clone()]
        } else {
            opts.watch_paths.clone()
        };
        let file_watcher = FsWatcher::start(&watch_roots, &default_ignore()).context("could not start file watcher")?;
        let watcher_rx = file_watcher.rx;
        let watcher_dispatcher = dispatcher.clone();
        let watcher_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let watcher_stop_thread = watcher_stop.clone();
        let watcher_thread = std::thread::spawn(move || {
            while !watcher_stop_thread.load(std::sync::atomic::Ordering::Relaxed) {
                let fs_event = match watcher_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(fs_event) => fs_event,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                };
                let event = Event {
                    event_id: String::new(),
                    session_id: String::new(),
                    project_id: String::new(),
                    sequence: 0,
                    source: EventSource::Other("fs".into()),
                    kind: "fs.event".into(),
                    severity: Severity::Info,
                    utc_timestamp: Utc::now(),
                    monotonic_ns: monotonic_now_ns(),
                    duration_ns: None,
                    correlation_id: None,
                    parent_event_id: None,
                    payload_version: 1,
                    payload: serde_json::to_value(&fs_event).unwrap_or_default(),
                    classification: PrivacyClassification::Internal,
                    previous_hash: None,
                    current_hash: None,
                };
                if watcher_dispatcher.emit(event).is_err() {
                    break;
                }
            }
        });

        let profiler = if opts.enable_profiler {
            Some(Profiler::start(
                Duration::from_millis(opts.profiler_interval_ms),
                dispatcher.clone(),
            ))
        } else {
            None
        };

        let pty_outcome = pty::run_pty(
            &opts.command,
            &opts.args,
            &opts.env,
            &opts.working_dir,
            dispatcher.clone(),
            opts.test_adapter_hint,
        )
        .await
        .context("failed to run command in PTY")?;

        if let Some(p) = profiler {
            p.stop().await;
        }

        watcher_stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = watcher_thread.join();

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
        .ok();

        let events_vec = self.repo.list_events(&session_id).unwrap_or_default();
        let final_count = events_vec.len() as u64;
        let redaction_total: u64 = self
            .repo
            .list_redactions(&session_id)
            .map(|f| f.len() as u64)
            .unwrap_or(0);

        let mut final_state = if pty_outcome.exit_status.success() {
            SessionState::Complete
        } else {
            SessionState::Failed
        };
        if opts.fail_on_findings && redaction_total > 0 {
            final_state = SessionState::Failed;
            warn!(
                findings = redaction_total,
                "session failed by user policy: redaction findings non-zero"
            );
        }

        self.repo
            .update_session_state(&session_id, final_state.clone(), Some(Utc::now()), None, final_count)?;

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

fn upsert_project(repo: &Repository, root: &Path) -> Result<ProjectInfo> {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let canonical_str = canonical.to_string_lossy().into_owned();

    let recent = repo.list_recent_sessions(40).unwrap_or_default();
    for sess in recent {
        let Ok(Some(p)) = repo.get_project(&sess.project_id) else {
            continue;
        };
        if p.root == canonical_str {
            return Ok(p);
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
    repo.ensure_project(&project)?;
    Ok(repo.get_project(&project_id)?.unwrap_or(project))
}

fn emit_core(dispatcher: &Dispatcher, kind: &str, severity: Severity, payload: serde_json::Value) -> Result<()> {
    let now = Utc::now();
    let event = Event {
        event_id: String::new(),
        session_id: String::new(),
        project_id: String::new(),
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
    dispatcher.emit(event)?;
    Ok(())
}
