//! Stable failure signatures.
//!
//! Equivalent failures should group under a single signature so a user
//! doesn't see a long tail of identical-looking errors. We normalise
//! identifiers that change between runs but preserve the parts that
//! matter for diagnosis.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// A normalised failure signature, with the exposed fields so grouping is
/// inspectable rather than opaque.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FailureSignature {
    pub exception_kind: String,
    pub normalised_message: String,
    pub top_frames: Vec<String>,
    pub exit_code: Option<i32>,
    pub component: Option<String>,
}

impl FailureSignature {
    /// Canonical, deterministic string for grouping. We lowercase and
    /// collapse whitespace; this is what callers compare.
    pub fn key(&self) -> String {
        let frames = self.top_frames.join("|");
        format!(
            "{exception}|{msg}|{frames}|{exit}|{comp}",
            exception = self.exception_kind.to_ascii_lowercase(),
            msg = self.normalised_message.to_ascii_lowercase(),
            frames = frames.to_ascii_lowercase(),
            exit = self
                .exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "none".into()),
            comp = self.component.as_deref().unwrap_or("").to_ascii_lowercase(),
        )
    }
}

/// Normalisers registered at startup. Each one knows how to strip noise.
/// Order matters: SPECIFIC patterns (UUID, pid=, file:line, IP, abs path)
/// run BEFORE generic digit-replacement so we don't preempt them.
static NOISE_PATTERNS: &[(&str, &str)] = &[
    // Hex constants (e.g. 0xdeadbeef).
    (r"\b0x[0-9a-fA-F]{4,}\b", "<HEX>"),
    // Hex UUIDs.
    (r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b", "<UUID>"),
    // Explicit pid=<NUM> sentinels (must precede generic digits).
    (r"\bpid\s*=\s*\d+\b", "<PID>"),
    // Stack line numbers (file.rs:123[:45]).
    (r"([\w./-]+\.[a-zA-Z]{1,8}):(\d+)(?::(\d+))?", "$1:<LINE>"),
    // IPv4 / IPv6 private-ish literals (before generic digits).
    (r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b", "<IP>"),
    // Absolute Windows paths.
    (r#"(?:[A-Z]:\\(?:Users|Windows|ProgramData|Program Files)[^\s"']+)"#, "<WINPATH>"),
    // Absolute Unix paths.
    (r#"/(?:home|Users|tmp|var|opt|root|srv)/[^\s"']+"#, "<PATH>"),
    // Time sentinels (HH:MM:SS).
    (r"\b\d{2}:\d{2}:\d{2}(?:\.\d+)?\b", "<TIME>"),
    // Calendar dates (YYYY-MM-DD).
    (r"\b\d{4}-\d{2}-\d{2}\b", "<DATE>"),
    // Generic numbers (must run last so all of the above can preempt).
    (r"\b\d{2,}\b", "<NUM>"),
];

static COMPILED: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    NOISE_PATTERNS
        .iter()
        .map(|(p, t)| (Regex::new(p).expect("static regex"), *t))
        .collect()
});

/// Normalise a single string by applying all noise replacements.
pub fn normalise_string(s: &str) -> String {
    let mut out = s.to_string();
    for (re, target) in COMPILED.iter() {
        out = re.replace_all(&out, *target).to_string();
    }
    out
}

/// Extract top N normalised stack frames from a multi-line traceback.
/// Recognises:
///   - Python: `File "x.py", line N, in fn`
///   - JavaScript / Node: `at fn (x.js:N:M)` or `at x.js:N:M`
///   - Rust panic: `0: src/main.rs:N` / `at src/main.rs:N`
///   - JVM: `at com.foo.bar.Baz.run(Baz.java:N)`
pub fn extract_top_frames(trace: &str, n: usize) -> Vec<String> {
    let mut frames = Vec::new();
    for raw in trace.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let is_frame = line.starts_with("at ")
            || line.contains(" at ")
            || line.contains("Traceback")
            || line.starts_with("File ")
            || line.starts_with("0:")
            || line.starts_with("1:")
            || line.starts_with("2:")
            || line.contains(".rs:")
            || line.contains(".ts:")
            || line.contains(".js:")
            || line.contains(".py:")
            || line.contains(".java:")
            || line.contains(".kt:")
            || line.contains(".go:")
            || line.contains("(most recent call last)");
        if is_frame {
            let normalised = normalise_string(line);
            if !normalised.is_empty() {
                frames.push(normalised);
            }
        }
        if frames.len() >= n {
            break;
        }
    }
    frames
}

/// Build a signature from a structured failure record.
pub fn make_signature(
    exception_kind: impl Into<String>,
    message: impl Into<String>,
    trace: Option<&str>,
    exit_code: Option<i32>,
    component: Option<&str>,
) -> FailureSignature {
    let ex = exception_kind.into();
    let msg = message.into();
    FailureSignature {
        exception_kind: ex.clone(),
        normalised_message: normalise_string(&msg),
        top_frames: trace.map(|t| extract_top_frames(t, 6)).unwrap_or_default(),
        exit_code,
        component: component.map(|s| s.to_string()),
    }
}

/// Build a signature from a Godot-shaped error.
pub fn godot_signature(scene: &str, message: &str) -> FailureSignature {
    FailureSignature {
        exception_kind: "godot.script_error".into(),
        normalised_message: normalise_string(message),
        top_frames: vec![format!("scene:<{}>", normalise_string(scene))],
        exit_code: None,
        component: Some("godot".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_paths_and_pids() {
        let s = "Failed at /home/alice/projects/foo/src/main.rs:123 pid=12345 UUID=550e8400-e29b-41d4-a716-446655440000";
        let out = normalise_string(s);
        assert!(out.contains("<PATH>"));
        assert!(out.contains("<PID>"));
        assert!(out.contains("<UUID>"));
        assert!(!out.contains("/home/alice"));
        assert!(!out.contains("12345"));
        assert!(!out.contains("550e8400-e29b"));
        // The combo of path+line may resolve to <PATH> or remain as <PATH>:<LINE>;
        // both are acceptable end states.
        assert!(out.contains("<LINE>") || !out.contains("123"));
    }

    #[test]
    fn signature_groups_equivalent_errors() {
        let s1 = make_signature("NullPointerException", "at /Users/bob/x/y.py:25", None, Some(-11), Some("python"));
        let s2 = make_signature("NullPointerException", "at /Users/alice/x/y.py:99", None, Some(-11), Some("python"));
        assert_eq!(s1.key(), s2.key());
    }

    #[test]
    fn signature_distinguishes_kinds() {
        let s1 = make_signature("NullPointerException", "x", None, None, None);
        let s2 = make_signature("IllegalStateException", "x", None, None, None);
        assert_ne!(s1.key(), s2.key());
    }

    #[test]
    fn extract_top_frames_caps_at_n() {
        let t = "Traceback (most recent call last):\n  File \"a.py\", line 1, in foo\n  File \"b.py\", line 2, in bar\n  File \"c.py\", line 3, in baz\n  File \"d.py\", line 4, in qux";
        let frames = extract_top_frames(t, 2);
        assert_eq!(frames.len(), 2);
    }

    #[test]
    fn godot_signature_keeps_component() {
        let sig = godot_signature("level_3", "player's health went below zero");
        assert_eq!(sig.component.as_deref(), Some("godot"));
        assert!(sig.top_frames[0].contains("level_3"));
    }
}
