use crate::identifier::Identifier;
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionState {
    Preparing,
    Recording,
    Stopping,
    Complete,
    Failed,
    Interrupted,
    ImportedReadOnly,
}

impl SessionState {
    pub const ALL: &'static [SessionState] = &[
        Self::Preparing,
        Self::Recording,
        Self::Stopping,
        Self::Complete,
        Self::Failed,
        Self::Interrupted,
        Self::ImportedReadOnly,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Preparing => "preparing",
            Self::Recording => "recording",
            Self::Stopping => "stopping",
            Self::Complete => "complete",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
            Self::ImportedReadOnly => "imported-read-only",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "preparing" => Some(Self::Preparing),
            "recording" => Some(Self::Recording),
            "stopping" => Some(Self::Stopping),
            "complete" => Some(Self::Complete),
            "failed" => Some(Self::Failed),
            "interrupted" => Some(Self::Interrupted),
            "imported-read-only" => Some(Self::ImportedReadOnly),
            _ => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Failed | Self::ImportedReadOnly)
    }
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "value")]
pub enum EventSource {
    Core,
    Cli,
    Vscode,
    Cursor,
    Windsurf,
    VSCodium,
    JetBrains,
    Neovim,
    Vim,
    VisualStudio,
    Sublime,
    Eclipse,
    Helix,
    Emacs,
    Nano,
    Godot,
    Agent,
    Mcp,
    Zed,
    RollingRecorder,
    TestAdapter,
    BundleImporter,
    Daemon,
    Browser,
    Proxy,
    Plugin,
    Sdk,
    Query,
    Other(String),
}

impl EventSource {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Core => "core",
            Self::Cli => "cli",
            Self::Vscode => "vscode",
            Self::Cursor => "cursor",
            Self::Windsurf => "windsurf",
            Self::VSCodium => "vscodium",
            Self::JetBrains => "jetbrains",
            Self::Neovim => "neovim",
            Self::Vim => "vim",
            Self::VisualStudio => "visual-studio",
            Self::Sublime => "sublime",
            Self::Eclipse => "eclipse",
            Self::Helix => "helix",
            Self::Emacs => "emacs",
            Self::Nano => "nano",
            Self::Godot => "godot",
            Self::Agent => "agent",
            Self::Mcp => "mcp",
            Self::Zed => "zed",
            Self::RollingRecorder => "rolling-recorder",
            Self::TestAdapter => "test-adapter",
            Self::BundleImporter => "bundle-importer",
            Self::Daemon => "daemon",
            Self::Browser => "browser",
            Self::Proxy => "proxy",
            Self::Plugin => "plugin",
            Self::Sdk => "sdk",
            Self::Query => "query",
            Self::Other(s) => s.as_str(),
        }
    }
}

impl fmt::Display for EventSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrivacyClassification {
    Unclassified,
    Public,
    Internal,
    Sensitive,
    Confidential,
}

impl PrivacyClassification {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unclassified => "unclassified",
            Self::Public => "public",
            Self::Internal => "internal",
            Self::Sensitive => "sensitive",
            Self::Confidential => "confidential",
        }
    }
}

impl fmt::Display for PrivacyClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Fatal,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Fatal => "fatal",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub session_id: String,
    pub project_id: String,
    pub sequence: u64,
    pub source: EventSource,
    pub kind: String,
    pub severity: Severity,
    pub utc_timestamp: DateTime<Utc>,
    pub monotonic_ns: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ns: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_event_id: Option<String>,
    pub payload_version: u32,
    pub payload: serde_json::Value,
    pub classification: PrivacyClassification,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_hash: Option<String>,
}

impl Event {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        event_id: Identifier,
        session_id: Identifier,
        project_id: Identifier,
        sequence: u64,
        source: EventSource,
        kind: impl Into<String>,
        severity: Severity,
        utc_timestamp: DateTime<Utc>,
        monotonic_ns: u64,
        payload_version: u32,
        payload: serde_json::Value,
        classification: PrivacyClassification,
    ) -> Self {
        Self {
            event_id: event_id.to_string(),
            session_id: session_id.to_string(),
            project_id: project_id.to_string(),
            sequence,
            source,
            kind: kind.into(),
            severity,
            utc_timestamp,
            monotonic_ns,
            duration_ns: None,
            correlation_id: None,
            parent_event_id: None,
            payload_version,
            payload,
            classification,
            previous_hash: None,
            current_hash: None,
        }
    }

    pub fn is_chain_sealed(&self) -> bool {
        self.current_hash.is_some()
    }

    pub fn is_error_like(&self) -> bool {
        matches!(self.severity, Severity::Error | Severity::Fatal)
            || self.kind.ends_with(".errored")
            || self.kind.ends_with("_failed")
            || self.kind.contains("failure")
            || self.kind.contains("error")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub project_id: String,
    pub name: String,
    pub root: String,
    pub language_hints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub project_id: String,
    pub state: SessionState,
    pub started_at: DateTime<Utc>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub labels: Vec<String>,
    pub source_event_count: u64,
    pub imported: bool,
    pub bundle_origin: Option<String>,
}

#[cfg(test)]
pub mod test_support {
    use super::{Event, EventSource, PrivacyClassification, Severity};
    use chrono::{TimeZone, Utc};

    pub fn make_blueprint_event() -> Event {
        let e = Event {
            event_id: "01h000000000000000000000aa".into(),
            session_id: "01h000000000000000000000ab".into(),
            project_id: "01h000000000000000000000ac".into(),
            sequence: 0,
            source: EventSource::Core,
            kind: "session.started".into(),
            severity: Severity::Info,
            utc_timestamp: Utc.timestamp_opt(1_700_000_000, 0).single().unwrap(),
            monotonic_ns: 0,
            duration_ns: None,
            correlation_id: None,
            parent_event_id: None,
            payload_version: 1,
            payload: serde_json::json!({
                "session_id": "01h000000000000000000000ab",
                "command": null,
                "args": []
            }),
            classification: PrivacyClassification::Internal,
            previous_hash: None,
            current_hash: None,
        };
        e
    }

    pub const BLUEPRINT_HASH_0: &str = "d0a30ea0cb259a3638626e7bc59abe6bc33885e020a2b6a8b4001b66eac0a21a";
}

impl Event {
    pub fn sample_at(seq: u64, ts_secs: i64, nsec: u32) -> Self {
        let ts = Utc.timestamp_opt(ts_secs, nsec).single().unwrap_or_else(Utc::now);
        let mut e = Event {
            event_id: format!("01H{seq:025}"),
            session_id: "01H00000000000000000000001".into(),
            project_id: "01H00000000000000000000002".into(),
            sequence: seq,
            source: EventSource::Core,
            kind: "test.sample".into(),
            severity: Severity::Info,
            utc_timestamp: ts,
            monotonic_ns: ts.timestamp_nanos_opt().unwrap_or_default().max(0) as u64,
            duration_ns: None,
            correlation_id: None,
            parent_event_id: None,
            payload_version: 1,
            payload: serde_json::json!({"sequence": seq}),
            classification: PrivacyClassification::Internal,
            previous_hash: None,
            current_hash: None,
        };
        e.event_id = synthesise_event_id(seq);
        e
    }
}

fn synthesise_event_id(seq: u64) -> String {
    use ulid::Ulid;
    // Deterministic clock/randomness so test ids are stable across runs.
    let ts_ms = 1_700_000_000_000u64 + seq * 100;
    let rand: u128 = ((seq.wrapping_add(0xc0ffee_babe)) as u128) | 1u128;
    Ulid::from_parts(ts_ms, rand).to_string().to_ascii_lowercase()
}
