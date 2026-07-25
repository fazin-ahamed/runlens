use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: &str = "runlens.bundle@1.0.0";

pub const COMPATIBLE_VERSIONS: &[&str] = &[
    "runlens.bundle@1.0.0",
    "runlens.bundle@1",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub format_version: String,
    pub exporter: ExporterInfo,
    pub project: ManifestProject,
    pub session: ManifestSession,
    pub event_count: u64,
    pub head_hash: Option<String>,
    pub invariants: InvariantSection,
    pub redacted_event_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExporterInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestProject {
    pub project_id: String,
    pub name: String,
    pub root: String,
    pub language_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSession {
    pub session_id: String,
    pub state: String,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub labels: Vec<String>,
    pub source_event_count: u64,
    pub imported: bool,
    pub bundle_origin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantSection {
    pub head_hash: Option<String>,
    pub event_count: Option<u64>,
    pub payload_hash: Option<String>,
}

pub fn is_compatible(version: &str) -> bool {
    COMPATIBLE_VERSIONS
        .iter()
        .any(|v| version.starts_with(v) || version == *v)
}