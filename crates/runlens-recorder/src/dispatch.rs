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
    max_events: Option<u64>,
}

impl Dispatcher {
    pub fn new(
        repo: Repository,
        project_id: String,
        session_id: String,
        initial_prev_hash: String,
        max_events: Option<u64>,
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
                max_events,
            }),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.inner.session_id
    }
    pub fn project_id(&self) -> &str {
        &self.inner.project_id
    }

    pub fn emit(&self, mut event: Event) -> anyhow::Result<Event> {
        event.utc_timestamp = Utc::now();

        let (mut event, findings) = self.inner.redactor.process_event(event);

        {
            let mut prev = self.inner.prev_hash.lock().unwrap();
            let mut seq = self.inner.next_sequence.lock().unwrap();
            if let Some(max) = self.inner.max_events {
                if *seq >= max {
                    return Err(anyhow::anyhow!("event cap reached: {max}"));
                }
            }
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
            let source_is_empty_other = matches!(&event.source, EventSource::Other(s) if s.is_empty());
            if source_is_empty_other {
                event.source = EventSource::Core;
            }

            let new_hash = chain::seal(&mut event, &prev);
            *prev = new_hash;
        }

        self.inner.repo.append_event(&event).context("append_event")?;

        for finding in &findings {
            if let Err(e) = self.inner.repo.record_redaction(
                &self.inner.session_id,
                Some(&event.event_id),
                finding.kind.as_str(),
                Some((finding.span_start, finding.span_end)),
                &finding.redaction,
                &finding.preview,
            ) {
                tracing::warn!(error=%e, "failed to persist redaction finding");
            }
        }

        trace!(
            session = %self.inner.session_id,
            "event sealed"
        );
        Ok(event)
    }
}

pub fn monotonic_now_ns() -> u64 {
    static ORIGIN: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let origin = ORIGIN.get_or_init(Instant::now);
    origin.elapsed().as_nanos() as u64
}
