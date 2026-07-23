use crate::model::{Event, EventSource, PrivacyClassification, Severity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// OpenTelemetry-inspired span kind for trace context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpanKind {
    Internal,
    Server,
    Client,
    Producer,
    Consumer,
}

impl SpanKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Internal => "internal",
            Self::Server => "server",
            Self::Client => "client",
            Self::Producer => "producer",
            Self::Consumer => "consumer",
        }
    }
}

/// Source-side clock information for cross-process ordering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceClock {
    pub clock_id: String,
    pub timestamp_ms: u64,
}

/// V2 event envelope with extended trace context, span support, and
/// correlation tracking. Backward-compatible: can be built from an
/// existing v1 Event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventV2 {
    pub event_id: String,
    pub session_id: String,
    pub project_id: String,
    pub sequence: u64,
    pub source: EventSource,
    pub kind: String,
    pub severity: Severity,
    pub utc_timestamp: DateTime<Utc>,
    pub monotonic_ns: u64,
    pub source_clock: Option<SourceClock>,
    pub duration_ns: Option<i64>,
    pub thread_id: Option<String>,
    pub task_id: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
    pub span_kind: Option<SpanKind>,
    pub correlation_id: Option<String>,
    pub parent_event_id: Option<String>,
    pub correlation_ids: Vec<String>,
    pub payload_version: u32,
    pub payload: serde_json::Value,
    pub classification: PrivacyClassification,
    pub previous_hash: Option<String>,
    pub current_hash: Option<String>,
    pub envelope_version: u32,
    pub schema_version: u32,
}

impl From<EventV2> for Event {
    fn from(ev2: EventV2) -> Self {
        Self {
            event_id: ev2.event_id,
            session_id: ev2.session_id,
            project_id: ev2.project_id,
            sequence: ev2.sequence,
            source: ev2.source,
            kind: ev2.kind,
            severity: ev2.severity,
            utc_timestamp: ev2.utc_timestamp,
            monotonic_ns: ev2.monotonic_ns,
            duration_ns: ev2.duration_ns,
            correlation_id: ev2.correlation_id,
            parent_event_id: ev2.parent_event_id,
            payload_version: ev2.payload_version,
            payload: ev2.payload,
            classification: ev2.classification,
            previous_hash: ev2.previous_hash,
            current_hash: ev2.current_hash,
        }
    }
}

impl EventV2 {
    pub const fn envelope_version() -> u32 {
        2
    }

    pub const fn schema_version() -> u32 {
        1
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: crate::identifier::Identifier,
        session_id: crate::identifier::Identifier,
        project_id: crate::identifier::Identifier,
        sequence: u64,
        source: EventSource,
        kind: impl Into<String>,
        severity: Severity,
        payload: serde_json::Value,
        classification: PrivacyClassification,
    ) -> Self {
        let now = Utc::now();
        Self {
            event_id: event_id.to_string(),
            session_id: session_id.to_string(),
            project_id: project_id.to_string(),
            sequence,
            source,
            kind: kind.into(),
            severity,
            utc_timestamp: now,
            monotonic_ns: now.timestamp_nanos_opt().unwrap_or(0).max(0) as u64,
            source_clock: None,
            duration_ns: None,
            thread_id: None,
            task_id: None,
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            span_kind: None,
            correlation_id: None,
            parent_event_id: None,
            correlation_ids: Vec::new(),
            payload_version: 1,
            payload,
            classification,
            previous_hash: None,
            current_hash: None,
            envelope_version: Self::envelope_version(),
            schema_version: Self::schema_version(),
        }
    }

    /// Convert from a v1 Event, filling extended fields with defaults.
    /// This lets v1 events pass through v2 pipelines losslessly.
    pub fn from_v1(event: Event) -> Self {
        Self {
            event_id: event.event_id,
            session_id: event.session_id,
            project_id: event.project_id,
            sequence: event.sequence,
            source: event.source,
            kind: event.kind,
            severity: event.severity,
            utc_timestamp: event.utc_timestamp,
            monotonic_ns: event.monotonic_ns,
            source_clock: None,
            duration_ns: event.duration_ns,
            thread_id: None,
            task_id: None,
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            span_kind: None,
            correlation_id: event.correlation_id,
            parent_event_id: event.parent_event_id,
            correlation_ids: Vec::new(),
            payload_version: event.payload_version,
            payload: event.payload,
            classification: event.classification,
            previous_hash: event.previous_hash,
            current_hash: event.current_hash,
            envelope_version: Self::envelope_version(),
            schema_version: Self::schema_version(),
        }
    }
}
