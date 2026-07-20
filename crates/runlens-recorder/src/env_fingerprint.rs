//! Privacy-safe environment fingerprint.
//!
//! Two parts:
//!   * Allow-listed keys whose VALUES are recorded (OS names, container
//!     hints, language versions). Anything not on the allow-list is
//!     dropped without ever being inspected.
//!   * Custom `RUNLENS_*` keys are always recorded as VALUES — those
//!     are explicitly user-controlled and safe by definition.
//!
//! For each captured (key, value) the fingerprint stores the BLAKE3
//! hash of the value plus a coarse category (`os`, `lang`, `ci`,
//! `custom`) so drift can be detected without leaking the value.

use blake3::Hasher;
use indexmap::IndexMap;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EnvFingerprint {
    pub captured: Vec<CapturedEntry>,
    pub redacted_classifications: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapturedEntry {
    pub category: EnvCategory,
    pub key: String,
    pub value_hash: String,
    pub value_preview: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvCategory {
    Os,
    Lang,
    Ci,
    Custom,
    Excluded,
}

const ALLOWED_OS: &[&str] = &[
    "PATH",
    "OS",
    "OSTYPE",
    "MSYSTEM",
    "PROCESSOR_ARCHITECTURE",
    "HOME",
    "HOMEDRIVE",
    "USERPROFILE",
    "TMPDIR",
    "TMP",
    "TEMP",
    "LANG",
    "LC_ALL",
    "TZ",
];

const ALLOWED_LANG: &[&str] = &[
    "JAVA_HOME",
    "PYTHON_VERSION",
    "NODE_VERSION",
    "GO_VERSION",
    "RUST_VERSION",
    "CARGO_HOME",
    "RUSTUP_HOME",
];

const ALLOWED_CI: &[&str] = &[
    "CI",
    "GITHUB_ACTIONS",
    "GITLAB_CI",
    "CIRCLECI",
    "BUILDKITE",
    "JENKINS_URL",
    "BUILDKITE_BUILD_ID",
    "GITHUB_RUN_ID",
    "GITHUB_RUN_NUMBER",
];

pub fn capture_env_fingerprint(env: &IndexMap<String, String>) -> EnvFingerprint {
    let mut captured = Vec::new();
    let _redacted_unused_marker: Vec<String> = Vec::new();

    // First, walk caller-supplied env (typically the child process's env).
    for (k, v) in env {
        if let Some(cat) = classify_key(k) {
            captured.push(build_entry(cat, k, v));
            continue;
        }
        if runlens_owns(k) {
            captured.push(build_entry(EnvCategory::Custom, k, v));
            continue;
        }
        // Off-list. Mark as Excluded and never record the value.
        captured.push(CapturedEntry {
            category: EnvCategory::Excluded,
            key: k.clone(),
            value_hash: "excluded".to_string(),
            value_preview: None,
        });
    }

    // env comes from the caller, not process::env()
    // recorded.

    let _ = _redacted_unused_marker; // (reserved for future value-classifier hits)
    EnvFingerprint {
        captured,
        redacted_classifications: vec![],
    }
}

fn classify_key(k: &str) -> Option<EnvCategory> {
    if ALLOWED_OS.iter().any(|a| a.eq_ignore_ascii_case(k)) {
        return Some(EnvCategory::Os);
    }
    if ALLOWED_LANG.iter().any(|a| a.eq_ignore_ascii_case(k)) {
        return Some(EnvCategory::Lang);
    }
    if ALLOWED_CI.iter().any(|a| a.eq_ignore_ascii_case(k)) {
        return Some(EnvCategory::Ci);
    }
    None
}

fn runlens_owns(k: &str) -> bool {
    k.starts_with("RUNLENS_")
}

fn build_entry(category: EnvCategory, key: &str, value: &str) -> CapturedEntry {
    let mut hasher = Hasher::new();
    hasher.update(key.as_bytes());
    hasher.update(b":");
    hasher.update(value.as_bytes());
    let h = hasher.finalize();
    let hex = format!(
        "blake3:{}",
        &h.to_hex()[..32.min(h.to_hex().len())]
    );
    let preview = if value.len() <= 32 && value.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
        Some(value.to_string())
    } else {
        None
    };
    CapturedEntry {
        category,
        key: key.to_string(),
        value_hash: hex,
        value_preview: preview,
    }
}

/// Convenience: filter the OS process env to just the caller-supplied
/// allow-list plus custom RUNLENS_* keys.
pub fn filter_process_env_to_fingerprint(
    process_env: impl Iterator<Item = (String, String)>,
) -> IndexMap<String, String> {
    let mut out: IndexMap<String, String> = IndexMap::new();
    for (k, v) in process_env {
        if classify_key(&k).is_some() || runlens_owns(&k) {
            out.insert(k, v);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_recognises_all_categories() {
        assert!(matches!(classify_key("PATH"), Some(EnvCategory::Os)));
        assert!(matches!(classify_key("NODE_VERSION"), Some(EnvCategory::Lang)));
        assert!(matches!(classify_key("GITHUB_ACTIONS"), Some(EnvCategory::Ci)));
        assert!(classify_key("AWS_SECRET_ACCESS_KEY").is_none());
    }

    #[test]
    fn runlens_keys_are_passed_through() {
        let mut env = IndexMap::new();
        env.insert("RUNLENS_PROJECT".to_string(), "demo".to_string());
        env.insert("AWS_SECRET".to_string(), "should be excluded".to_string());
        let fp = capture_env_fingerprint(&env);
        assert!(fp
            .captured
            .iter()
            .any(|e| e.key == "RUNLENS_PROJECT" && matches!(e.category, EnvCategory::Custom)));
        assert!(fp
            .captured
            .iter()
            .any(|e| e.key == "AWS_SECRET" && matches!(e.category, EnvCategory::Excluded)));
    }

    #[test]
    fn preview_is_suppressed_for_long_strings() {
        let mut env = IndexMap::new();
        env.insert("RUNLENS_NOTE".to_string(), "a".repeat(120));
        let fp = capture_env_fingerprint(&env);
        let entry = fp
            .captured
            .iter()
            .find(|e| e.key == "RUNLENS_NOTE")
            .expect("entry");
        assert!(entry.value_preview.is_none());
    }

    #[test]
    fn hash_changes_when_value_changes() {
        let mut env_a = IndexMap::new();
        env_a.insert("RUNLENS_BUILD".to_string(), "v1".to_string());
        let mut env_b = IndexMap::new();
        env_b.insert("RUNLENS_BUILD".to_string(), "v2".to_string());
        let fp_a = capture_env_fingerprint(&env_a);
        let fp_b = capture_env_fingerprint(&env_b);
        let ha = &fp_a.captured[0].value_hash;
        let hb = &fp_b.captured[0].value_hash;
        assert_ne!(ha, hb);
    }
}
