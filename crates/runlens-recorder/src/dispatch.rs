//! In-process pub/sub that connects collectors to the storage repo and
//! optional in-memory consumers.
//!
//! Collectors do not write to SQLite directly. Each collector accepts a
//! [`Dispatcher`] and calls `dispatch.emit(Event)`. The dispatcher is
//! responsible for: validating the event, applying privacy findings,
//! sealing the hash chain against the previous event, and finally
//! persisting the row to storage.
//!
//! The dispatcher is single-thread for the emit pathway (a `Mutex`) so the
//! hash chain order is deterministic. Real collectors can run in parallel
//! via `tokio::spawn`; only the emit call contends on the inner lock.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Context;
use chrono::Utc;
use runlens_core::chain;
use runlens_core::identifier::Identifier;
use runlens_core::model::{Event, EventSource, PrivacyClassification};
use runlens_storage::Repository;
use tracing::trace;

use crate::redaction::Redactor;

/// Shared handle that collectors use to publish events.
#[derive(Clone)]
pub struct Dispatcher {
    inner: Arc<DispatcherInner>,
}

struct DispatcherInner {
    repo: Repository,
    session_id: String,
    project_id: String,
    redactor: Redactor,
    ulid_gen: Mutex<ulid::Generator>,
    prev_hash: Mutex<String>,
    next_sequence: Mutex<u64>,
}

impl Dispatcher {
    /// Build a dispatcher anchored on a known session.
    pub fn new(
        repo: Repository,
        project_id: String,
        session_id: String,
        initial_prev_hash: String,
    ) -> Self {
        Self {
            inner: Arc::new(DispatcherInner {
                repo,
                session_id,
                project_id,
                redactor: Redactor::default(),
                ulid_gen: Mutex::new(ulid::Generator::new()),
                prev_hash: Mutex::new(initial_prev_hash),
                next_sequence: Mutex::new(0),
            }),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.inner.session_id
    }
    pub fn project_id(&self) -> &str {
        &self.inner.project_id
    }

    /// Emit a fully constructed event. The dispatcher:
    ///   * assigns / increments sequence number
    ///   * assigns event_id (ULID via `monotonic`)
    ///   * stamps monotonic_ns from a monotonic clock
    ///   * seals the event into the BLAKE3 chain
    ///   * persists the row to SQLite
    ///
    /// Returns the sealed event as written.
    pub async fn emit(&self, mut event: Event) -> anyhow::Result<Event> {
        event.utc_timestamp = Utc::now();

        // 1. Apply redactions first so redaction findings can be echoed
        //    back. Findings are not persisted in this codepath; the
        //    redacted payload is what hits disk.
        let (mut event, _findings) = self.inner.redactor.process_event(event);

        {
            let mut prev = self.inner.prev_hash.lock().unwrap();
            let mut seq = self.inner.next_sequence.lock().unwrap();
            let mut gen = self.inner.ulid_gen.lock().unwrap();

            event.sequence = *seq;
            *seq += 1;

            if event.event_id.is_empty() {
                let id = Identifier::monotonic(&mut gen);
                event.event_id = id.as_str().to_string();
            }
            event.session_id = self.inner.session_id.clone();
            event.project_id = self.inner.project_id.clone();

            if matches!(event.classification, PrivacyClassification::Unclassified) {
                event.classification = PrivacyClassification::Internal;
            }
            // Source-default: if it's EventSource::Other("") we treat as Core.
            let source_is_empty_other = matches!(&event.source, EventSource::Other(s) if s.is_empty());
            if source_is_empty_other {
                event.source = EventSource::Core;
            }

            let new_hash = chain::seal(&mut event, &prev);
            *prev = new_hash;
        }

        self.inner
            .repo
            .append_event(&event)
            .await
            .context("append_event")?;

        trace!(
            session = %self.inner.session_id,
            "event sealed"
        );
        Ok(event)
    }
}

/// Return monotonic now in nanoseconds since an arbitrary origin.
/// Uses parking_lot-free std monotonic clock.
pub fn monotonic_now_ns() -> u64 {
    static ORIGIN: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let origin = ORIGIN.get_or_init(Instant::now);
    origin.elapsed().as_nanos() as u64
}
