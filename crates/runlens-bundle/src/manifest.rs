//! Bundle manifest format + version compatibility.

use serde::{Deserialize, Serialize};

/// Current on-disk bundle format identifier. Every emitted bundle writes
/// `FORMAT_VERSION` as `bundle_format_version` in `bundle.toml`.
pub const FORMAT_VERSION: &str = "runlens.bundle@1.0.0";

/// Vector of format-version substrings that this build accepts on import.
/// Anything not listed here is rejected as `UnsupportedBundleVersion`.
pub const COMPATIBLE_VERSIONS: &[&str] = &[
    "runlens.bundle@1.0.0",
    "runlens.bundle@1", // forward-compatible major-version prefix
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BundleManifest {
    pub bundle_format_version: String,
    pub exporter: ExporterInfo,
    pub project: ManifestProject,
    pub session: ManifestSession,
    pub event_count: u64,
    pub byte_count_total: u64,
    pub invariants: InvariantSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExporterInfo {
    pub tool: String,
    pub tool_version: String,
    pub host_arch: String,
    pub host_os: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestProject {
    pub project_id: String,
    pub name: String,
    pub root_masked: String,
    pub language_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestSession {
    pub session_id: String,
    pub state: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub labels: Vec<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub stopped_at: Option<chrono::DateTime<chrono::Utc>>,
    pub exit_code: Option<i32>,
    pub source_event_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvariantSection {
    pub genesis_hash: String,
    pub head_hash: Option<String>,
    pub verify_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArtifactRef {
    pub hash: String,
    pub size: u64,
    pub origin: Option<String>,
    pub media_kind: Option<String>,
}

/// Return true if `version` is in `COMPATIBLE_VERSIONS` or is in the
/// forward-compatible family.
pub fn is_compatible(version: &str) -> bool {
    if COMPATIBLE_VERSIONS.iter().any(|v| *v == version) {
        return true;
    }
    if version.starts_with("runlens.bundle@") {
        // Any 1.x.y is forward-compatible given the major version.
        let major = version
            .trim_start_matches("runlens.bundle@")
            .split('.')
            .next()
            .unwrap_or("");
        if major == "1" {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_compatible_accepts_exact_match_and_major_prefix() {
        assert!(is_compatible("runlens.bundle@1.0.0"));
        assert!(is_compatible("runlens.bundle@1.2.3"));
        assert!(is_compatible("runlens.bundle@1"));
        assert!(!is_compatible("runlens.bundle@0.9.0"));
        assert!(!is_compatible("unknown@1.0.0"));
    }
}
