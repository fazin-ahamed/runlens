use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use runlens_core::model::{
    Event, EventSource, PrivacyClassification, ProjectInfo, SessionInfo, SessionState, Severity,
};
use rusqlite::{params, Connection};

use crate::error::{StorageError, StorageResult};
use crate::migrations;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RedactionFinding {
    pub kind: String,
    pub preview: String,
}

#[derive(Clone)]
pub struct Repository {
    conn: Arc<Mutex<Connection>>,
}

impl Repository {
    pub fn open(path: impl AsRef<std::path::Path>) -> StorageResult<Self> {
        let conn = Connection::open(path.as_ref())?;
        apply_pragmas(&conn)?;
        migrations::run(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn in_memory() -> StorageResult<Self> {
        let conn = Connection::open_in_memory()?;
        apply_pragmas(&conn)?;
        migrations::run(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn conn(&self) -> &Arc<Mutex<Connection>> {
        &self.conn
    }

    pub fn insert_event(&self, event: &Event) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO event (id, session_id, project_id, sequence, source, kind, severity, utc_timestamp, monotonic_ns, duration_ns, correlation_id, parent_event_id, payload_version, payload_json, classification, previous_hash, current_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                event.event_id,
                event.session_id,
                event.project_id,
                event.sequence,
                event.source.to_string(),
                event.kind,
                event.severity.to_string(),
                event.utc_timestamp.to_rfc3339(),
                event.monotonic_ns,
                event.duration_ns,
                event.correlation_id,
                event.parent_event_id,
                event.payload_version,
                serde_json::to_string(&event.payload)?,
                event.classification.to_string(),
                event.previous_hash,
                event.current_hash,
            ],
        )?;
        Ok(())
    }

    pub fn append_event(&self, event: &Event) -> StorageResult<()> {
        self.insert_event(event)
    }

    pub fn batch_append_events(&self, events: &[Event]) -> StorageResult<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let mut session_ids: Vec<&str> = Vec::new();
        for event in events {
            tx.execute(
                "INSERT INTO event (id, session_id, project_id, sequence, source, kind, severity, utc_timestamp, monotonic_ns, duration_ns, correlation_id, parent_event_id, payload_version, payload_json, classification, previous_hash, current_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    event.event_id,
                    event.session_id,
                    event.project_id,
                    event.sequence,
                    event.source.to_string(),
                    event.kind,
                    event.severity.to_string(),
                    event.utc_timestamp.to_rfc3339(),
                    event.monotonic_ns,
                    event.duration_ns,
                    event.correlation_id,
                    event.parent_event_id,
                    event.payload_version,
                    serde_json::to_string(&event.payload)?,
                    event.classification.to_string(),
                    event.previous_hash,
                    event.current_hash,
                ],
            )?;
            if !session_ids.contains(&event.session_id.as_str()) {
                session_ids.push(event.session_id.as_str());
            }
        }
        for sid in &session_ids {
            let count: u64 = tx.query_row(
                "SELECT COUNT(*) FROM event WHERE session_id = ?1",
                params![sid],
                |row| row.get(0),
            )?;
            tx.execute(
                "UPDATE session SET source_event_count = ?1 WHERE id = ?2",
                params![count, sid],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_event(&self, id: &str) -> StorageResult<Event> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, session_id, project_id, sequence, source, kind, severity, utc_timestamp, monotonic_ns, duration_ns, correlation_id, parent_event_id, payload_version, payload_json, classification, previous_hash, current_hash
             FROM event WHERE id = ?1",
            params![id],
            row_to_event,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => StorageError::EventNotFound(id.to_string()),
            other => StorageError::Sqlite(other),
        })
    }

    pub fn list_events(&self, session_id: &str) -> StorageResult<Vec<Event>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, project_id, sequence, source, kind, severity, utc_timestamp, monotonic_ns, duration_ns, correlation_id, parent_event_id, payload_version, payload_json, classification, previous_hash, current_hash
             FROM event WHERE session_id = ?1 ORDER BY sequence",
        )?;

        let rows = stmt.query_map(params![session_id], row_to_event)?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    pub fn events_by_session(&self, session_id: &str) -> StorageResult<Vec<Event>> {
        self.list_events(session_id)
    }

    pub fn create_session(&self, info: &SessionInfo) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO session (id, project_id, state, started_at, stopped_at, command, args_json, labels_json, source_event_count, imported, bundle_origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                info.session_id,
                info.project_id,
                info.state.to_string(),
                info.started_at.to_rfc3339(),
                info.stopped_at.map(|t| t.to_rfc3339()),
                info.command,
                serde_json::to_string(&info.args)?,
                serde_json::to_string(&info.labels)?,
                info.source_event_count,
                info.imported,
                info.bundle_origin,
            ],
        )?;
        Ok(())
    }

    pub fn update_session_state(
        &self,
        session_id: &str,
        state: SessionState,
        stopped_at: Option<DateTime<Utc>>,
        _command: Option<&str>,
        event_count: u64,
    ) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE session SET state = ?1, stopped_at = ?2, source_event_count = ?3 WHERE id = ?4",
            params![
                state.to_string(),
                stopped_at.map(|t| t.to_rfc3339()),
                event_count,
                session_id,
            ],
        )?;
        Ok(())
    }

    pub fn get_session(&self, session_id: &str) -> StorageResult<SessionInfo> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, project_id, state, started_at, stopped_at, command, args_json, labels_json, source_event_count, imported, bundle_origin
             FROM session WHERE id = ?1",
            params![session_id],
            row_to_session,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                StorageError::EventNotFound(session_id.to_string())
            }
            other => StorageError::Sqlite(other),
        })
    }

    pub fn list_recent_sessions(&self, limit: usize) -> StorageResult<Vec<SessionInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, state, started_at, stopped_at, command, args_json, labels_json, source_event_count, imported, bundle_origin
             FROM session ORDER BY started_at DESC LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], row_to_session)?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    pub fn ensure_project(&self, info: &ProjectInfo) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM project WHERE id = ?1",
                params![info.project_id],
                |r| r.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if !exists {
            conn.execute(
                "INSERT INTO project (id, name, root, language_hints_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    info.project_id,
                    info.name,
                    info.root,
                    serde_json::to_string(&info.language_hints)?,
                ],
            )?;
        }
        Ok(())
    }

    pub fn get_project(&self, project_id: &str) -> StorageResult<Option<ProjectInfo>> {
        let conn = self.conn.lock().unwrap();
        let found = conn.query_row(
            "SELECT id, name, root, language_hints_json FROM project WHERE id = ?1",
            params![project_id],
            |row| {
                let hints_json: String = row.get(3)?;
                let language_hints: Vec<String> = serde_json::from_str(&hints_json)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok(ProjectInfo {
                    project_id: row.get(0)?,
                    name: row.get(1)?,
                    root: row.get(2)?,
                    language_hints,
                })
            },
        );

        match found {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Sqlite(e)),
        }
    }

    pub fn record_redaction(
        &self,
        session_id: &str,
        event_id: Option<&str>,
        kind: &str,
        span: Option<(usize, usize)>,
        redaction: &str,
        preview: &str,
    ) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO redaction (session_id, event_id, kind, span_start, span_end, redaction, preview)
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

    pub fn list_redactions(&self, session_id: &str) -> StorageResult<Vec<RedactionFinding>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT kind, preview FROM redaction WHERE session_id = ?1")?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(RedactionFinding {
                kind: row.get(0)?,
                preview: row.get(1)?,
            })
        })?;
        let mut findings = Vec::new();
        for row in rows {
            findings.push(row?);
        }
        Ok(findings)
    }
}

fn row_to_event(row: &rusqlite::Row) -> rusqlite::Result<Event> {
    let ts_str: String = row.get(7)?;
    let ts: DateTime<Utc> = DateTime::parse_from_rfc3339(&ts_str)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
        .into();
    let payload_json: String = row.get(13)?;
    let payload: serde_json::Value =
        serde_json::from_str(&payload_json).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    Ok(Event {
        event_id: row.get(0)?,
        session_id: row.get(1)?,
        project_id: row.get(2)?,
        sequence: row.get(3)?,
        source: parse_source(&row.get::<_, String>(4)?),
        kind: row.get(5)?,
        severity: parse_severity(&row.get::<_, String>(6)?),
        utc_timestamp: ts,
        monotonic_ns: row.get(8)?,
        duration_ns: row.get(9)?,
        correlation_id: row.get(10)?,
        parent_event_id: row.get(11)?,
        payload_version: row.get(12)?,
        payload,
        classification: parse_classification(&row.get::<_, String>(14)?),
        previous_hash: row.get(15)?,
        current_hash: row.get(16)?,
    })
}

fn row_to_session(row: &rusqlite::Row) -> rusqlite::Result<SessionInfo> {
    let args_json: String = row.get(6)?;
    let args: Vec<String> =
        serde_json::from_str(&args_json).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let labels_json: String = row.get(7)?;
    let labels: Vec<String> =
        serde_json::from_str(&labels_json).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let stopped_at_str: Option<String> = row.get(4)?;
    let stopped_at = stopped_at_str
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    Ok(SessionInfo {
        session_id: row.get(0)?,
        project_id: row.get(1)?,
        state: parse_session_state(&row.get::<_, String>(2)?),
        started_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
            .into(),
        stopped_at,
        command: row.get(5)?,
        args,
        labels,
        source_event_count: row.get(8)?,
        imported: row.get(9)?,
        bundle_origin: row.get(10)?,
    })
}

fn parse_source(s: &str) -> EventSource {
    match s {
        "core" => EventSource::Core,
        "cli" => EventSource::Cli,
        "vscode" => EventSource::Vscode,
        "cursor" => EventSource::Cursor,
        "windsurf" => EventSource::Windsurf,
        "vscodium" => EventSource::VSCodium,
        "jetbrains" => EventSource::JetBrains,
        "neovim" => EventSource::Neovim,
        "vim" => EventSource::Vim,
        "visual-studio" => EventSource::VisualStudio,
        "sublime" => EventSource::Sublime,
        "eclipse" => EventSource::Eclipse,
        "helix" => EventSource::Helix,
        "emacs" => EventSource::Emacs,
        "nano" => EventSource::Nano,
        "godot" => EventSource::Godot,
        "agent" => EventSource::Agent,
        "mcp" => EventSource::Mcp,
        "zed" => EventSource::Zed,
        "rolling-recorder" => EventSource::RollingRecorder,
        "test-adapter" => EventSource::TestAdapter,
        "bundle-importer" => EventSource::BundleImporter,
        "daemon" => EventSource::Daemon,
        "browser" => EventSource::Browser,
        "proxy" => EventSource::Proxy,
        "plugin" => EventSource::Plugin,
        "sdk" => EventSource::Sdk,
        "query" => EventSource::Query,
        other => EventSource::Other(other.to_string()),
    }
}

fn parse_severity(s: &str) -> Severity {
    match s {
        "info" => Severity::Info,
        "warning" => Severity::Warning,
        "error" => Severity::Error,
        "fatal" => Severity::Fatal,
        _ => Severity::Info,
    }
}

fn parse_classification(s: &str) -> PrivacyClassification {
    match s {
        "unclassified" => PrivacyClassification::Unclassified,
        "public" => PrivacyClassification::Public,
        "internal" => PrivacyClassification::Internal,
        "sensitive" => PrivacyClassification::Sensitive,
        "confidential" => PrivacyClassification::Confidential,
        _ => PrivacyClassification::Unclassified,
    }
}

fn parse_session_state(s: &str) -> SessionState {
    match s {
        "preparing" => SessionState::Preparing,
        "recording" => SessionState::Recording,
        "stopping" => SessionState::Stopping,
        "complete" => SessionState::Complete,
        "failed" => SessionState::Failed,
        "interrupted" => SessionState::Interrupted,
        "imported-read-only" => SessionState::ImportedReadOnly,
        _ => SessionState::Failed,
    }
}

fn apply_pragmas(conn: &Connection) -> StorageResult<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use runlens_core::identifier::Identifier;

    fn sample_event(sequence: u64) -> Event {
        Event::build(
            Identifier::now(),
            Identifier::now(),
            Identifier::now(),
            sequence,
            EventSource::Cli,
            "test",
            Severity::Info,
            Utc::now(),
            1_000_000,
            1,
            serde_json::json!({"msg": "hello"}),
            PrivacyClassification::Public,
        )
    }

    #[test]
    fn insert_and_retrieve() {
        let repo = Repository::in_memory().unwrap();
        let event = sample_event(1);
        repo.insert_event(&event).unwrap();

        let got = repo.get_event(&event.event_id).unwrap();
        assert_eq!(got.event_id, event.event_id);
        assert_eq!(got.sequence, 1);
        assert_eq!(got.kind, "test");
        assert_eq!(got.source, EventSource::Cli);
    }

    #[test]
    fn get_event_not_found() {
        let repo = Repository::in_memory().unwrap();
        let err = repo.get_event("nonexistent").unwrap_err();
        assert!(matches!(err, StorageError::EventNotFound(_)));
    }

    #[test]
    fn events_by_session_ordered() {
        let repo = Repository::in_memory().unwrap();
        let session = Identifier::now();
        let mut ids = Vec::new();

        for seq in 0..5 {
            let mut event = sample_event(seq);
            event.session_id = session.to_string();
            repo.insert_event(&event).unwrap();
            ids.push(event.event_id.clone());
        }

        let events = repo.events_by_session(&session.to_string()).unwrap();
        assert_eq!(events.len(), 5);
        for (i, e) in events.iter().enumerate() {
            assert_eq!(e.sequence, i as u64);
            assert_eq!(e.session_id, session.to_string());
        }
    }

    #[test]
    fn create_and_get_session() {
        let repo = Repository::in_memory().unwrap();
        let p = ProjectInfo {
            project_id: "test-project".into(),
            name: "test".into(),
            root: "/tmp".into(),
            language_hints: vec![],
        };
        repo.ensure_project(&p).unwrap();
        let info = SessionInfo {
            session_id: "test-session".into(),
            project_id: "test-project".into(),
            state: SessionState::Preparing,
            started_at: Utc::now(),
            stopped_at: None,
            command: Some("echo".into()),
            args: vec!["hello".into()],
            labels: vec!["dev".into()],
            source_event_count: 0,
            imported: false,
            bundle_origin: None,
        };
        repo.create_session(&info).unwrap();

        let got = repo.get_session("test-session").unwrap();
        assert_eq!(got.session_id, "test-session");
        assert_eq!(got.state, SessionState::Preparing);
    }

    #[test]
    fn ensure_and_get_project() {
        let repo = Repository::in_memory().unwrap();
        let info = ProjectInfo {
            project_id: "proj-1".into(),
            name: "test".into(),
            root: "/tmp/test".into(),
            language_hints: vec!["rust".into()],
        };
        repo.ensure_project(&info).unwrap();
        let got = repo.get_project("proj-1").unwrap().unwrap();
        assert_eq!(got.name, "test");
    }

    #[test]
    fn list_recent_sessions() {
        let repo = Repository::in_memory().unwrap();
        let p = ProjectInfo {
            project_id: "p".into(),
            name: "test".into(),
            root: "/tmp".into(),
            language_hints: vec![],
        };
        repo.ensure_project(&p).unwrap();
        for i in 0..3 {
            let info = SessionInfo {
                session_id: format!("sess-{i}"),
                project_id: "p".into(),
                state: SessionState::Complete,
                started_at: Utc::now(),
                stopped_at: Some(Utc::now()),
                command: None,
                args: vec![],
                labels: vec![],
                source_event_count: 0,
                imported: false,
                bundle_origin: None,
            };
            repo.create_session(&info).unwrap();
        }
        let sessions = repo.list_recent_sessions(2).unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn clone_is_cheap() {
        let repo = Repository::in_memory().unwrap();
        let r2 = repo.clone();
        let event = sample_event(0);
        r2.insert_event(&event).unwrap();
        repo.get_event(&event.event_id).unwrap();
    }
}
