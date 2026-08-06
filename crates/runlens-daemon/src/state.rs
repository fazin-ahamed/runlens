use crate::pipeline::IngestHandle;
use crate::subscription::SubscriptionManager;
use chrono::{DateTime, Utc};
use runlens_storage::repo::Repository;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};

static NEXT_SESSION_NUM: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
}

pub struct DaemonState {
    pub started_at: DateTime<Utc>,
    pub db_path: String,
    pub active_sessions: Arc<RwLock<HashMap<String, ActiveSession>>>,
    pub shutdown: Arc<Notify>,
    pub ingest: IngestHandle,
    pub subscriptions: Arc<SubscriptionManager>,
    pub repo: Repository,
}

impl DaemonState {
    pub fn new(
        db_path: String,
        ingest: IngestHandle,
        subscriptions: Arc<SubscriptionManager>,
        repo: Repository,
    ) -> Self {
        Self {
            started_at: Utc::now(),
            db_path,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            shutdown: Arc::new(Notify::new()),
            ingest,
            subscriptions,
            repo,
        }
    }

    pub fn next_session_num() -> u64 {
        NEXT_SESSION_NUM.fetch_add(1, Ordering::SeqCst)
    }

    pub fn uptime_secs(&self) -> u64 {
        let elapsed = Utc::now() - self.started_at;
        elapsed.num_seconds().max(0) as u64
    }

    pub async fn session_count(&self) -> usize {
        self.active_sessions.read().await.len()
    }

    pub async fn register_session(&self, session_id: String) {
        let session = ActiveSession {
            session_id: session_id.clone(),
            started_at: Utc::now(),
        };
        self.active_sessions.write().await.insert(session_id, session);
    }

    pub async fn unregister_session(&self, session_id: &str) {
        self.active_sessions.write().await.remove(session_id);
    }

    pub fn signal_shutdown(&self) {
        self.shutdown.notify_one();
    }
}
