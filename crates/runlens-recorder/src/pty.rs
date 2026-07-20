//! Cross-platform PTY child execution.
//!
//! Spawns `command` with the given args, env, and working directory inside a
//! pseudo-terminal so line buffering / TTY-driven progress bars behave as on a
//! real terminal. Read-only capture only.
//!
//! Output is shipped to the [`crate::dispatch::Dispatcher`] as a stream of
//! chunked `pty.stdout` events. Each chunk is first scanned by the redactor
//! before the chunk is sealed.

use std::io::Read;
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use runlens_core::model::{Event, EventSource, PrivacyClassification, Severity};
use runlens_core::signatures::godot_signature;
use serde::Serialize;
use tracing::debug;
use tokio::runtime::Handle;

use crate::dispatch::Dispatcher;
use crate::redaction::Redactor;
use crate::test_adapters::{detect_adapter, run_adapter, TestAdapterHint};

#[derive(Debug, Clone)]
pub struct PtyOutcome {
    pub exit_status: ExitStatus,
    pub wall_clock_ms: u64,
    pub test_summary: TestSummary,
}

#[derive(Debug, Clone)]
pub struct ExitStatus {
    pub code: Option<i32>,
}

impl ExitStatus {
    pub fn success(&self) -> bool {
        matches!(self.code, Some(0))
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TestSummary {
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub inconclusive: u32,
}

pub async fn run_pty(
    command: &str,
    args: &[String],
    env: &indexmap::IndexMap<String, String>,
    working_dir: &Path,
    dispatcher: Dispatcher,
    test_adapter: Option<crate::session::TestAdapterKind>,
) -> Result<PtyOutcome> {
    let command = command.to_string();
    let args = args.to_vec();
    let env = env.clone();
    let working_dir = working_dir.to_path_buf();
    let hint = test_adapter
        .map(|k| match k {
            crate::session::TestAdapterKind::Junit => TestAdapterHint::Junit,
            crate::session::TestAdapterKind::Pytest => TestAdapterHint::Pytest,
            crate::session::TestAdapterKind::Vitest => TestAdapterHint::Vitest,
            crate::session::TestAdapterKind::Gotest => TestAdapterHint::Gotest,
        })
        .unwrap_or(TestAdapterHint::Auto);

    tokio::task::spawn_blocking(move || {
        run_blocking(&command, &args, &env, &working_dir, dispatcher, hint)
    })
    .await
    .context("PTY worker join")?
}

fn run_blocking(
    command: &str,
    args: &[String],
    env: &indexmap::IndexMap<String, String>,
    working_dir: &Path,
    dispatcher: Dispatcher,
    test_adapter: TestAdapterHint,
) -> Result<PtyOutcome> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("openpty")?;

    let mut cmd = CommandBuilder::new(command);
    for a in args {
        cmd.arg(a);
    }
    cmd.cwd(working_dir);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.env("TERM", "xterm-256color");
    if std::env::var_os("NO_COLOR").is_some() {
        cmd.env("FORCE_COLOR", "0");
    } else {
        cmd.env("FORCE_COLOR", "1");
    }
    cmd.env("CI", "1");

    let started_at = Instant::now();
    let mut child = pair.slave.spawn_command(cmd).context("spawn child")?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .context("clone reader")?;

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let reader_thread = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    debug!(error=%e, "pty read error");
                    break;
                }
            }
        }
    });

    let redactor = Redactor::default();
    let mut adapter = detect_adapter(test_adapter);
    let mut test_summary = TestSummary::default();
    let mut combined: Vec<u8> = Vec::new();

    loop {
        match rx.try_recv() {
            Ok(chunk) => {
                combined.extend_from_slice(&chunk);
                run_adapter(&mut adapter, &chunk, &mut test_summary);
                emit_pty_chunk(&dispatcher, &chunk, &redactor);
            }
            Err(mpsc::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                // Drain any remaining buffered output before we exit.
                while let Ok(chunk) = rx.try_recv() {
                    combined.extend_from_slice(&chunk);
                    run_adapter(&mut adapter, &chunk, &mut test_summary);
                    emit_pty_chunk(&dispatcher, &chunk, &redactor);
                }
                let exit_status = ExitStatus {
                    code: Some(status.exit_code() as i32),
                };
                let _ = reader_thread.join();

                let wall_clock_ms = started_at.elapsed().as_millis() as u64;

                let trace = std::str::from_utf8(&combined).unwrap_or("");
                let trimmed = trace.lines().rev().take(40).collect::<Vec<_>>().join("\n");
                let summary_text = if let Some(code) = exit_status.code {
                    if code == 0 {
                        String::new()
                    } else {
                        format!("exit {code}")
                    }
                } else {
                    String::new()
                };
                if !summary_text.is_empty() {
                    let sig = godot_signature(&trimmed, &summary_text);
                    let json = serde_json::to_value(&sig).unwrap_or_default();
                    let event = Event {
                        event_id: String::new(),
                        session_id: dispatcher.session_id().to_string(),
                        project_id: dispatcher.project_id().to_string(),
                        sequence: 0,
                        source: EventSource::Other("pty".into()),
                        kind: "failure.signature".into(),
                        severity: Severity::Error,
                        utc_timestamp: Utc::now(),
                        monotonic_ns: crate::dispatch::monotonic_now_ns(),
                        duration_ns: Some(wall_clock_ms.min(i64::MAX as u64) as i64 * 1_000_000),
                        correlation_id: None,
                        parent_event_id: None,
                        payload_version: 1,
                        payload: json,
                        classification: PrivacyClassification::Internal,
                        previous_hash: None,
                        current_hash: None,
                    };
                    Handle::current().block_on(dispatcher.emit(event)).ok();
                }

                return Ok(PtyOutcome {
                    exit_status,
                    wall_clock_ms,
                    test_summary,
                });
            }
            Ok(None) => continue,
            Err(e) => return Err(anyhow!("try_wait: {e}")),
        }
    }

    Err(anyhow!("pty loop exited without child status"))
}

fn emit_pty_chunk(dispatcher: &Dispatcher, bytes: &[u8], redactor: &Redactor) {
    let text = String::from_utf8_lossy(bytes).to_string();
    let redacted_value = redactor.process_event(Event {
        event_id: String::new(),
        session_id: String::new(),
        project_id: String::new(),
        sequence: 0,
        source: EventSource::Other("pty".into()),
        kind: "pty.stdout".into(),
        severity: Severity::Info,
        utc_timestamp: Utc::now(),
        monotonic_ns: 0,
        duration_ns: None,
        correlation_id: None,
        parent_event_id: None,
        payload_version: 1,
        payload: serde_json::Value::String(text.clone()),
        classification: PrivacyClassification::Sensitive,
        previous_hash: None,
        current_hash: None,
    });
    let text = match redacted_value.0.payload {
        serde_json::Value::String(s) => s,
        _ => text,
    };
    let event = Event {
        event_id: String::new(),
        session_id: dispatcher.session_id().to_string(),
        project_id: dispatcher.project_id().to_string(),
        sequence: 0,
        source: EventSource::Other("pty".into()),
        kind: "pty.stdout".into(),
        severity: Severity::Info,
        utc_timestamp: Utc::now(),
        monotonic_ns: crate::dispatch::monotonic_now_ns(),
        duration_ns: None,
        correlation_id: None,
        parent_event_id: None,
        payload_version: 1,
        payload: serde_json::json!({"text": text, "len": bytes.len()}),
        classification: PrivacyClassification::Sensitive,
        previous_hash: None,
        current_hash: None,
    };
    Handle::current().block_on(dispatcher.emit(event)).ok();
}
