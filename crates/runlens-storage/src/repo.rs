//! Repository: opens a SQLite database, applies migrations, exposes typed
//! queries for projects, sessions, events, artifacts, markers,
//! redactions, comparisons, imports, investigations, and integrations.

use crate::migrations;
use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use runlens_core::model::{
    Event, EventSource, PrivacyClassification, ProjectInfo, SessionInfo, SessionState, Severity,
};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Wraps a Connection with default pragmas for performance + safety.
#[derive(Clone)]
pub struct Repository {
    conn: Arc<Mutex<Connection>>,
}

impl Repository {
    /// Open or create a repository at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path).context("opening sqlite")?;
        Self::tune(&conn)?;
        migrations::run(&conn).context("running migrations")?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    /// In-memory repository, useful for tests. WAL is disabled for in-memory.
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("opening in-memory sqlite")?;
        Self::tune_in_memory(&conn)?;
        migrations::run(&conn).context("running migrations")?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    fn tune(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;
             PRAGMA busy_timeout = 5000;",
        )?;
        Ok(())
    }

    fn tune_in_memory(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;",
        )?;
        Ok(())
    }

    /// Acquire the shared connection for a quick synchronous call.
    pub async fn conn(&self) -> tokio::sync::MutexGuard<'_, Connection> {
        self.conn.lock().await
    }

    pub async fn ensure_project(&self, info: &ProjectInfo) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO projects(project_id, name, root, language_hints) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id) DO UPDATE SET name=excluded.name, root=excluded.root, language_hints=excluded.language_hints",
            params![info.project_id, info.name, info.root, serde_json::to_string(&info.language_hints)?],
        )?;
        Ok(())
    }

    pub async fn get_project(&self, project_id: &str) -> Result<Option<ProjectInfo>> {
        let conn = self.conn.lock().await;
        let row = conn
            .query_row(
                "SELECT project_id, name, root, language_hints FROM projects WHERE project_id = ?1",
                params![project_id],
                |r| {
                    let langs: String = r.get(3)?;
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        langs,
                    ))
                },
            )
            .optional()?;
        Ok(row.map(|(id, name, root, langs)| ProjectInfo {
            project_id: id,
            name,
            root,
            language_hints: serde_json::from_str(&langs).unwrap_or_default(),
        }))
    }

    pub async fn create_session(&self, info: &SessionInfo) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO sessions(session_id, project_id, state, started_at, stopped_at, command, args, labels, source_event_count, imported, bundle_origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(session_id) DO UPDATE SET state=excluded.state, stopped_at=excluded.stopped_at, command=excluded.command, args=excluded.args, labels=excluded.labels, source_event_count=excluded.source_event_count, imported=excluded.imported, bundle_origin=excluded.bundle_origin",
            params![
                info.session_id,
                info.project_id,
                info.state.as_str(),
                info.started_at.to_rfc3339(),
                info.stopped_at.map(|t| t.to_rfc3339()),
                info.command,
                serde_json::to_string(&info.args)?,
                serde_json::to_string(&info.labels)?,
                info.source_event_count as i64,
                info.imported as i64,
                info.bundle_origin,
            ],
        )?;
        Ok(())
    }

    pub async fn update_session_state(
        &self,
        session_id: &str,
        state: SessionState,
        stopped_at: Option<chrono::DateTime<chrono::Utc>>,
        final_head_hash: Option<&str>,
        event_count: u64,
    ) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE sessions SET state = ?2, stopped_at = ?3, final_head_hash = ?4, source_event_count = ?5 WHERE session_id = ?1",
            params![session_id, state.as_str(), stopped_at.map(|t| t.to_rfc3339()), final_head_hash, event_count as i64],
        )?;
        Ok(())
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>> {
        let conn = self.conn.lock().await;
        let row = conn
            .query_row(
                "SELECT session_id, project_id, state, started_at, stopped_at, command, args, labels, source_event_count, imported, bundle_origin
                 FROM sessions WHERE session_id = ?1",
                params![session_id],
                |r| session_info_columns(r),
            )
            .optional()?;
        Ok(row.map(row_to_session_info))
    }

    pub async fn list_sessions_for_project(&self, project_id: &str, limit: u32) -> Result<Vec<SessionInfo>> {
        let conn = self.conn.lock().await;
        let rows = conn
            .prepare(
                "SELECT session_id, project_id, state, started_at, stopped_at, command, args, labels, source_event_count, imported, bundle_origin
                 FROM sessions WHERE project_id = ?1 ORDER BY started_at DESC LIMIT ?2",
            )?
            .query_map(params![project_id, limit as i64], |r| session_info_columns(r))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows.into_iter().map(row_to_session_info).collect())
    }

    pub async fn list_recent_sessions(&self, limit: u32) -> Result<Vec<SessionInfo>> {
        let conn = self.conn.lock().await;
        let rows = conn
            .prepare(
                "SELECT session_id, project_id, state, started_at, stopped_at, command, args, labels, source_event_count, imported, bundle_origin
                 FROM sessions ORDER BY started_at DESC LIMIT ?1",
            )?
            .query_map(params![limit as i64], |r| session_info_columns(r))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows.into_iter().map(row_to_session_info).collect())
    }

    pub async fn append_event(&self, event: &Event) -> Result<()> {
        let conn = self.conn.lock().await;
        let tx = conn.unchecked_transaction()?;
        let (source_kind, source_value) = match &event.source {
            EventSource::Other(s) => ("other".to_string(), s.clone()),
            other => (other.as_str().to_string(), String::new()),
        };
        tx.execute(
            "INSERT INTO events(event_id, session_id, project_id, sequence, source_kind, source_value, kind, severity, utc_timestamp, monotonic_ns, duration_ns, correlation_id, parent_event_id, payload_version, payload_json, classification, previous_hash, current_hash, is_error_like)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
             ON CONFLICT(event_id) DO NOTHING",
            params![
                event.event_id,
                event.session_id,
                event.project_id,
                event.sequence as i64,
                source_kind,
                source_value,
                event.kind,
                event.severity.as_str(),
                event.utc_timestamp.to_rfc3339(),
                event.monotonic_ns,
                event.duration_ns,
                event.correlation_id,
                event.parent_event_id,
                event.payload_version as i64,
                serde_json::to_string(&event.payload)?,
                event.classification.as_str(),
                event.previous_hash,
                event.current_hash,
                event.is_error_like() as i64,
            ],
        )?;
        tx.execute(
            "UPDATE sessions SET source_event_count = source_event_count + 1 WHERE session_id = ?1",
            params![event.session_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub async fn list_events(&self, session_id: &str) -> Result<Vec<Event>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT event_id, session_id, project_id, sequence, source_kind, source_value, kind, severity, utc_timestamp, monotonic_ns, duration_ns, correlation_id, parent_event_id, payload_version, payload_json, classification, previous_hash, current_hash
             FROM events WHERE session_id = ?1 ORDER BY sequence ASC",
        )?;
        let rows = stmt
            .query_map(params![session_id], |r| event_columns(r))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows.into_iter().map(row_to_event).collect())
    }

    pub async fn list_events_paged(
        &self,
        session_id: &str,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<Event>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT event_id, session_id, project_id, sequence, source_kind, source_value, kind, severity, utc_timestamp, monotonic_ns, duration_ns, correlation_id, parent_event_id, payload_version, payload_json, classification, previous_hash, current_hash
             FROM events WHERE session_id = ?1 ORDER BY sequence ASC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt
            .query_map(params![session_id, limit as i64, offset as i64], |r| event_columns(r))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows.into_iter().map(row_to_event).collect())
    }

    pub async fn last_sequence(&self, session_id: &str) -> Result<u64> {
        let conn = self.conn.lock().await;
        let n: Option<i64> = conn
            .query_row(
                "SELECT MAX(sequence) FROM events WHERE session_id = ?1",
                params![session_id],
                |r| r.get(0),
            )
            .optional()?;
        Ok(n.unwrap_or(0).max(0) as u64)
    }

    pub async fn search_events(
        &self,
        kind: Option<&str>,
        severity: Option<&str>,
        error_only: bool,
        project_id: Option<&str>,
        session_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<Event>> {
        let conn = self.conn.lock().await;
        let mut sql = String::from(
            "SELECT event_id, session_id, project_id, sequence, source_kind, source_value, kind, severity, utc_timestamp, monotonic_ns, duration_ns, correlation_id, parent_event_id, payload_version, payload_json, classification, previous_hash, current_hash FROM events WHERE 1=1",
        );
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(k) = kind {
            sql.push_str(" AND kind = ?");
            args.push(Box::new(k.to_string()));
        }
        if let Some(s) = severity {
            sql.push_str(" AND severity = ?");
            args.push(Box::new(s.to_string()));
        }
        if error_only {
            sql.push_str(" AND is_error_like = 1");
        }
        if let Some(p) = project_id {
            sql.push_str(" AND project_id = ?");
            args.push(Box::new(p.to_string()));
        }
        if let Some(s) = session_id {
            sql.push_str(" AND session_id = ?");
            args.push(Box::new(s.to_string()));
        }
        sql.push_str(" ORDER BY utc_timestamp DESC LIMIT ?");
        args.push(Box::new(limit as i64));
        let binds: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(binds), |r| event_columns(r))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows.into_iter().map(row_to_event).collect())
    }

    pub async fn record_redaction(
        &self,
        session_id: &str,
        event_id: Option<&str>,
        kind: &str,
        span: Option<(usize, usize)>,
        redaction: &str,
        preview: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO redaction_findings(session_id, event_id, kind, span_start, span_end, redaction, preview)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                session_id,
                event_id,
                kind,
                span.map(|s| s.0 as i64),
                span.map(|s| s.1 as i64),
                redaction,
                preview,
            ],
        )?;
        Ok(())
    }

    pub async fn list_redactions(&self, session_id: &str) -> Result<Vec<RedactionRow>> {
        let conn = self.conn.lock().await;
        let rows = conn
            .prepare(
                "SELECT finding_id, event_id, session_id, kind, span_start, span_end, redaction, preview, created_at
                 FROM redaction_findings WHERE session_id = ?1 ORDER BY finding_id ASC",
            )?
            .query_map(params![session_id], |r| {
                Ok(RedactionRow {
                    finding_id: r.get::<_, i64>(0)? as u64,
                    event_id: r.get(1)?,
                    session_id: r.get(2)?,
                    kind: r.get(3)?,
                    span_start: r.get::<_, Option<i64>>(4)?.map(|v| v as usize),
                    span_end: r.get::<_, Option<i64>>(5)?.map(|v| v as usize),
                    redaction: r.get(6)?,
                    preview: r.get(7)?,
                    created_at: r.get(8)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub async fn record_comparison(
        &self,
        id: &str,
        baseline: &str,
        candidate: &str,
        summary_json: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO comparisons(comparison_id, baseline_session_id, candidate_session_id, summary_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, baseline, candidate, summary_json],
        )?;
        Ok(())
    }

    pub async fn record_import(
        &self,
        id: &str,
        bundle_path: &str,
        size: u64,
        session_id: &str,
        verified: bool,
    ) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO imports(import_id, bundle_path, bundle_size, imported_session_id, verified) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, bundle_path, size as i64, session_id, verified as i64],
        )?;
        Ok(())
    }

    pub async fn find_interrupted_sessions(&self) -> Result<Vec<SessionInfo>> {
        let conn = self.conn.lock().await;
        let rows = conn
            .prepare(
                "SELECT session_id, project_id, state, started_at, stopped_at, command, args, labels, source_event_count, imported, bundle_origin
                 FROM sessions WHERE state IN ('recording','stopping','preparing') ORDER BY started_at ASC",
            )?
            .query_map([], |r| session_info_columns(r))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows.into_iter().map(row_to_session_info).collect())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RedactionRow {
    pub finding_id: u64,
    pub event_id: Option<String>,
    pub session_id: String,
    pub kind: String,
    pub span_start: Option<usize>,
    pub span_end: Option<usize>,
    pub redaction: String,
    pub preview: String,
    pub created_at: String,
}

// ====== Row decoders ======

fn session_info_columns(r: &rusqlite::Row<'_>) -> rusqlite::Result<SessionInfoRaw> {
    Ok(SessionInfoRaw {
        session_id: r.get(0)?,
        project_id: r.get(1)?,
        state: r.get(2)?,
        started_at: r.get(3)?,
        stopped_at: r.get(4)?,
        command: r.get(5)?,
        args: r.get(6)?,
        labels: r.get(7)?,
        source_event_count: r.get::<_, i64>(8)?,
        imported: r.get::<_, i64>(9)? != 0,
        bundle_origin: r.get(10)?,
    })
}

struct SessionInfoRaw {
    session_id: String,
    project_id: String,
    state: String,
    started_at: String,
    stopped_at: Option<String>,
    command: Option<String>,
    args: String,
    labels: String,
    source_event_count: i64,
    imported: bool,
    bundle_origin: Option<String>,
}

fn row_to_session_info(r: SessionInfoRaw) -> SessionInfo {
    let state = SessionState::parse(&r.state).unwrap_or(SessionState::Failed);
    let started_at = parse_rfc3339(&r.started_at).unwrap_or_else(chrono::Utc::now);
    let stopped_at = r.stopped_at.and_then(|s| parse_rfc3339(&s));
    SessionInfo {
        session_id: r.session_id,
        project_id: r.project_id,
        state,
        started_at,
        stopped_at,
        command: r.command,
        args: serde_json::from_str(&r.args).unwrap_or_default(),
        labels: serde_json::from_str(&r.labels).unwrap_or_default(),
        source_event_count: r.source_event_count.max(0) as u64,
        imported: r.imported,
        bundle_origin: r.bundle_origin,
    }
}

fn parse_rfc3339(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
}

fn event_columns(r: &rusqlite::Row<'_>) -> rusqlite::Result<EventRaw> {
    Ok(EventRaw {
        event_id: r.get(0)?,
        session_id: r.get(1)?,
        project_id: r.get(2)?,
        sequence: r.get::<_, i64>(3)?,
        source_kind: r.get(4)?,
        source_value: r.get(5)?,
        kind: r.get(6)?,
        severity: r.get(7)?,
        utc_timestamp: r.get(8)?,
        monotonic_ns: r.get::<_, i64>(9)?.max(0) as u64,
        duration_ns: r.get::<_, Option<i64>>(10)?,
        correlation_id: r.get(11)?,
        parent_event_id: r.get(12)?,
        payload_version: r.get::<_, i64>(13)?,
        payload_json: r.get(14)?,
        classification: r.get(15)?,
        previous_hash: r.get(16)?,
        current_hash: r.get(17)?,
    })
}

struct EventRaw {
    event_id: String,
    session_id: String,
    project_id: String,
    sequence: i64,
    source_kind: String,
    source_value: String,
    kind: String,
    severity: String,
    utc_timestamp: String,
    monotonic_ns: u64,
    duration_ns: Option<i64>,
    correlation_id: Option<String>,
    parent_event_id: Option<String>,
    payload_version: i64,
    payload_json: String,
    classification: String,
    previous_hash: Option<String>,
    current_hash: Option<String>,
}

fn row_to_event(r: EventRaw) -> Event {
    let source = match r.source_kind.as_str() {
        "core" => EventSource::Core,
        "cli" => EventSource::Cli,
        "vscode" => EventSource::Vscode,
        "godot" => EventSource::Godot,
        "agent" => EventSource::Agent,
        "mcp" => EventSource::Mcp,
        "zed" => EventSource::Zed,
        "rolling-recorder" => EventSource::RollingRecorder,
        "test-adapter" => EventSource::TestAdapter,
        "bundle-importer" => EventSource::BundleImporter,
        _ => EventSource::Other(r.source_value.clone()),
    };
    Event {
        event_id: r.event_id,
        session_id: r.session_id,
        project_id: r.project_id,
        sequence: r.sequence.max(0) as u64,
        source,
        kind: r.kind,
        severity: parse_severity(&r.severity),
        utc_timestamp: parse_rfc3339(&r.utc_timestamp).unwrap_or_else(chrono::Utc::now),
        monotonic_ns: r.monotonic_ns,
        duration_ns: r.duration_ns,
        correlation_id: r.correlation_id,
        parent_event_id: r.parent_event_id,
        payload_version: r.payload_version.max(0) as u32,
        payload: serde_json::from_str(&r.payload_json).unwrap_or(serde_json::Value::Null),
        classification: parse_classification(&r.classification),
        previous_hash: r.previous_hash,
        current_hash: r.current_hash,
    }
}

fn parse_severity(s: &str) -> Severity {
    match s {
        "warning" => Severity::Warning,
        "error" => Severity::Error,
        "fatal" => Severity::Fatal,
        _ => Severity::Info,
    }
}

fn parse_classification(s: &str) -> PrivacyClassification {
    match s {
        "public" => PrivacyClassification::Public,
        "sensitive" => PrivacyClassification::Sensitive,
        "confidential" => PrivacyClassification::Confidential,
        _ => PrivacyClassification::Internal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use runlens_core::identifier::Identifier;
    use runlens_core::model::Event;

    #[tokio::test]
    async fn round_trip_session_and_event() {
        let repo = Repository::in_memory().unwrap();
        let now = Utc::now();
        let project = ProjectInfo {
            project_id: Identifier::now().to_string(),
            name: "demo".into(),
            root: "/tmp/demo".into(),
            language_hints: vec!["rust".into()],
        };
        repo.ensure_project(&project).await.unwrap();
        let info = SessionInfo {
            session_id: Identifier::now().to_string(),
            project_id: project.project_id.clone(),
            state: SessionState::Recording,
            started_at: now,
            stopped_at: None,
            command: Some("cargo".into()),
            args: vec!["test".into()],
            labels: vec![],
            source_event_count: 0,
            imported: false,
            bundle_origin: None,
        };
        repo.create_session(&info).await.unwrap();
        let ev = Event::build(
            Identifier::now(),
            Identifier::from_string(&info.session_id).unwrap(),
            Identifier::from_string(&project.project_id).unwrap(),
            0,
            EventSource::Cli,
            "test.sample",
            Severity::Info,
            now,
            0,
            1,
            serde_json::json!({"a":1}),
            PrivacyClassification::Internal,
        );
        repo.append_event(&ev).await.unwrap();
        let fetched = repo.get_session(&info.session_id).await.unwrap().unwrap();
        assert_eq!(fetched.session_id, info.session_id);
        assert_eq!(fetched.state, SessionState::Recording);
        let events = repo.list_events(&info.session_id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, "test.sample");
    }

    #[tokio::test]
    async fn search_by_severity_works() {
        let repo = Repository::in_memory().unwrap();
        let project = ProjectInfo {
            project_id: Identifier::now().to_string(),
            name: "demo".into(),
            root: "/tmp/demo".into(),
            language_hints: vec![],
        };
        repo.ensure_project(&project).await.unwrap();
        let session = SessionInfo {
            session_id: Identifier::now().to_string(),
            project_id: project.project_id.clone(),
            state: SessionState::Recording,
            started_at: Utc::now(),
            stopped_at: None,
            command: None,
            args: vec![],
            labels: vec![],
            source_event_count: 0,
            imported: false,
            bundle_origin: None,
        };
        repo.create_session(&session).await.unwrap();
        let sid = Identifier::from_string(&session.session_id).unwrap();
        let pid = Identifier::from_string(&project.project_id).unwrap();
        for i in 0..5 {
            let severity = if i == 2 { Severity::Error } else { Severity::Info };
            let mut ev = Event::build(
                Identifier::now(),
                sid.clone(),
                pid.clone(),
                i,
                EventSource::Cli,
                "test.sample",
                severity,
                Utc::now(),
                0,
                1,
                serde_json::json!({"i": i}),
                PrivacyClassification::Internal,
            );
            ev.event_id = Identifier::now().to_string();
            repo.append_event(&ev).await.unwrap();
        }
        let errors = repo
            .search_events(None, Some("error"), false, None, None, 100)
            .await
            .unwrap();
        assert_eq!(errors.len(), 1);
        let error_only = repo
            .search_events(None, None, true, None, None, 100)
            .await
            .unwrap();
        assert_eq!(error_only.len(), 1);
    }
}
