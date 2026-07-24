use std::path::PathBuf;

use runlens_core::model::Event;
use runlens_storage::Repository;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RollingError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("io: {0}")]
    Io(String),
}

#[derive(Debug, Clone)]
pub struct RollingConfig {
    pub db_dir: PathBuf,
    pub max_events: u64,
    pub max_sessions: u32,
    pub rotate_threshold: u64,
}

impl Default for RollingConfig {
    fn default() -> Self {
        Self {
            db_dir: PathBuf::from(".runlens/rolling"),
            max_events: 100_000,
            max_sessions: 50,
            rotate_threshold: 80_000,
        }
    }
}

pub struct RollingRecorder {
    config: RollingConfig,
    active: Option<Repository>,
    session_count: u32,
}

impl RollingRecorder {
    pub fn new(config: RollingConfig) -> Self {
        Self {
            config,
            active: None,
            session_count: 0,
        }
    }

    pub fn open(&mut self) -> Result<(), RollingError> {
        let db_dir = &self.config.db_dir;
        std::fs::create_dir_all(db_dir).map_err(|e| RollingError::Io(e.to_string()))?;

        let mut db_path = db_dir.join("current.db");
        let mut attempt = 0;

        loop {
            if attempt > 0 {
                db_path = db_dir.join(format!("current.{attempt}.db"));
            }

            let repo = Repository::open(&db_path)
                .map_err(|e| RollingError::Storage(e.to_string()))?;

            let sessions = repo
                .list_recent_sessions(1)
                .map_err(|e| RollingError::Storage(e.to_string()))?;

            let total_events = if let Some(latest) = sessions.first() {
                self.session_count = 1;
                latest.source_event_count
            } else {
                0
            };

            if total_events >= self.config.rotate_threshold {
                attempt += 1;
                continue;
            }

            self.active = Some(repo);
            return Ok(());
        }
    }

    pub fn repo(&self) -> Option<&Repository> {
        self.active.as_ref()
    }

    pub fn insert_event(&self, event: &Event) -> Result<(), RollingError> {
        match &self.active {
            Some(repo) => repo
                .insert_event(event)
                .map_err(|e| RollingError::Storage(e.to_string())),
            None => Err(RollingError::Storage("no active database".into())),
        }
    }

    pub fn should_rotate(&self) -> bool {
        self.active
            .as_ref()
            .and_then(|r| {
                r.list_recent_sessions(1)
                    .ok()
                    .and_then(|s| s.first().map(|s| s.source_event_count))
            })
            .map(|c| c >= self.config.rotate_threshold)
            .unwrap_or(false)
    }

    pub fn rotate(&mut self) -> Result<(), RollingError> {
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let archive_name = format!("archive.{timestamp}.db");
        let archive_path = self.config.db_dir.join(&archive_name);

        let current_path = self.config.db_dir.join("current.db");
        std::fs::rename(&current_path, &archive_path)
            .map_err(|e| RollingError::Io(e.to_string()))?;

        self.cleanup_old()?;

        let repo = Repository::open(&current_path)
            .map_err(|e| RollingError::Storage(e.to_string()))?;
        self.active = Some(repo);
        self.session_count = 0;

        Ok(())
    }

    fn cleanup_old(&self) -> Result<(), RollingError> {
        let mut archives: Vec<_> = std::fs::read_dir(&self.config.db_dir)
            .map_err(|e| RollingError::Io(e.to_string()))?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("archive.") && n.ends_with(".db"))
                    .unwrap_or(false)
            })
            .collect();

        archives.sort_by_key(|e| e.path());

        let max_archives = self.config.max_sessions as usize;
        if archives.len() > max_archives {
            for old in archives.iter().take(archives.len() - max_archives) {
                std::fs::remove_file(old.path())
                    .map_err(|e| RollingError::Io(e.to_string()))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runlens_core::identifier::Identifier;
    use runlens_core::model::{EventSource, PrivacyClassification, Severity};
    use chrono::Utc;
    use tempfile::tempdir;

    fn sample_event() -> Event {
        Event::build(
            Identifier::now(),
            Identifier::now(),
            Identifier::now(),
            0,
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
    fn open_creates_db_dir() {
        let dir = tempdir().unwrap();
        let db_dir = dir.path().join("rolling");
        let mut r = RollingRecorder::new(RollingConfig {
            db_dir: db_dir.clone(),
            max_events: 100_000,
            max_sessions: 5,
            rotate_threshold: 80_000,
        });
        r.open().unwrap();
        assert!(db_dir.join("current.db").exists());
    }

    #[test]
    fn insert_and_query() {
        let dir = tempdir().unwrap();
        let mut r = RollingRecorder::new(RollingConfig {
            db_dir: dir.path().join("rolling"),
            max_events: 100_000,
            max_sessions: 5,
            rotate_threshold: 80_000,
        });
        r.open().unwrap();
        let event = sample_event();
        r.insert_event(&event).unwrap();
        let repo = r.repo().unwrap();
        let got = repo.get_event(&event.event_id).unwrap();
        assert_eq!(got.event_id, event.event_id);
    }

    #[test]
    fn should_rotate_returns_false_initially() {
        let dir = tempdir().unwrap();
        let mut r = RollingRecorder::new(RollingConfig {
            db_dir: dir.path().join("rolling"),
            max_events: 100_000,
            max_sessions: 5,
            rotate_threshold: 100,
        });
        r.open().unwrap();
        assert!(!r.should_rotate());
    }
}