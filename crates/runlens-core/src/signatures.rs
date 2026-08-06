use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FailureSignature {
    pub exception_kind: String,
    pub normalised_message: String,
    pub top_frames: Vec<String>,
    pub exit_code: Option<i32>,
    pub component: Option<String>,
}

impl FailureSignature {
    pub fn key(&self) -> String {
        let frames = self.top_frames.join("|");
        let mut key = String::with_capacity(64);
        key.push_str(&self.exception_kind.to_ascii_lowercase());
        key.push('|');
        key.push_str(&self.normalised_message.to_ascii_lowercase());
        key.push('|');
        key.push_str(&frames.to_ascii_lowercase());
        key.push('|');
        match self.exit_code {
            Some(code) => key.push_str(&code.to_string()),
            None => key.push_str("none"),
        }
        key.push('|');
        if let Some(component) = self.component.as_deref() {
            key.push_str(&component.to_ascii_lowercase());
        }
        key
    }
}

static NOISE_PATTERNS: &[(&str, &str)] = &[
    (r"\b0x[0-9a-fA-F]{4,}\b", "<HEX>"),
    (
        r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b",
        "<UUID>",
    ),
    (r"\bpid\s*=\s*\d+\b", "<PID>"),
    (r"([\w./-]+\.[a-zA-Z]{1,8}):(\d+)(?::(\d+))?", "$1:<LINE>"),
    (r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b", "<IP>"),
    (
        r#"(?:[A-Z]:\\(?:Users|Windows|ProgramData|Program Files)[^\s"']+)"#,
        "<WINPATH>",
    ),
    (r#"/(?:home|Users|tmp|var|opt|root|srv)/[^\s"']+"#, "<PATH>"),
    (r"\b\d{2}:\d{2}:\d{2}(?:\.\d+)?\b", "<TIME>"),
    (r"\b\d{4}-\d{2}-\d{2}\b", "<DATE>"),
    (r"\b\d{2,}\b", "<NUM>"),
];

static COMPILED: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    NOISE_PATTERNS
        .iter()
        .map(|(p, t)| (Regex::new(p).expect("static regex"), *t))
        .collect()
});

pub fn normalise_string(s: &str) -> String {
    let mut out = s.to_string();
    for (re, target) in COMPILED.iter() {
        out = re.replace_all(&out, *target).to_string();
    }
    out
}

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
        let s =
            "Failed at /home/alice/projects/foo/src/main.rs:123 pid=12345 UUID=550e8400-e29b-41d4-a716-446655440000";
        let out = normalise_string(s);
        assert!(out.contains("<PATH>"));
        assert!(out.contains("<PID>"));
        assert!(out.contains("<UUID>"));
        assert!(!out.contains("/home/alice"));
        assert!(!out.contains("12345"));
        assert!(!out.contains("550e8400-e29b"));
        assert!(out.contains("<LINE>") || !out.contains("123"));
    }

    #[test]
    fn signature_groups_equivalent_errors() {
        let python_error = make_signature(
            "NullPointerException",
            "at /Users/bob/x/y.py:25",
            None,
            Some(-11),
            Some("python"),
        );
        let twin_error = make_signature(
            "NullPointerException",
            "at /Users/alice/x/y.py:99",
            None,
            Some(-11),
            Some("python"),
        );
        assert_eq!(python_error.key(), twin_error.key());
    }

    #[test]
    fn signature_distinguishes_kinds() {
        let null_pointer = make_signature("NullPointerException", "x", None, None, None);
        let illegal_state = make_signature("IllegalStateException", "x", None, None, None);
        assert_ne!(null_pointer.key(), illegal_state.key());
    }

    #[test]
    fn extract_top_frames_caps_at_n() {
        let trace = "Traceback (most recent call last):\n  File \"a.py\", line 1, in foo\n  File \"b.py\", line 2, in bar\n  File \"c.py\", line 3, in baz\n  File \"d.py\", line 4, in qux";
        let frames = extract_top_frames(trace, 2);
        assert_eq!(frames.len(), 2);
    }

    #[test]
    fn godot_signature_keeps_component() {
        let sig = godot_signature("level_3", "player's health went below zero");
        assert_eq!(sig.component.as_deref(), Some("godot"));
        assert!(sig.top_frames[0].contains("level_3"));
    }
}
