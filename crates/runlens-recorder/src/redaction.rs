//! Bridges [`runlens_core::privacy`] with event payloads so collectors
//! don't have to remember to redact.
//!
//! The redactor scans event payloads for secrets and applies redactions.
//! Findings are returned so the caller can persist them via
//! [`runlens_storage::repo::Repo::record_redaction`].

use runlens_core::model::Event;
use runlens_core::privacy::{
    apply_redactions, default_patterns, scan_string, Finding, SecretPattern,
};
use serde_json::Value;

/// Stateful redactor. Constructed once per session so compiled patterns
/// are reused across many events.
#[derive(Clone)]
pub struct Redactor {
    patterns: Vec<SecretPattern>,
}

impl Default for Redactor {
    fn default() -> Self {
        Self {
            patterns: default_patterns(),
        }
    }
}

impl Redactor {
    pub fn new(patterns: Vec<SecretPattern>) -> Self {
        Self { patterns }
    }

    /// Process an event: walk payload, scan strings, apply redactions, collect
    /// findings. The redacted payload is what gets sealed and persisted.
    pub fn process_event(&self, mut event: Event) -> (Event, Vec<Finding>) {
        let mut findings: Vec<Finding> = Vec::new();
        event.payload = redact_recursive(
            event.payload.take(),
            &self.patterns,
            &mut findings,
        );
        (event, findings)
    }
}

fn redact_recursive(
    v: Value,
    patterns: &[SecretPattern],
    findings: &mut Vec<Finding>,
) -> Value {
    match v {
        Value::String(s) => {
            let (_, mut f) = scan_string(&s, patterns);
            if f.is_empty() {
                return Value::String(s);
            }
            f.sort_by_key(|x| (x.span_start, std::cmp::Reverse(x.span_end)));
            let mut kept: Vec<Finding> = Vec::with_capacity(f.len());
            let mut last_end: usize = 0;
            for finding in f {
                if finding.span_start >= last_end {
                    last_end = finding.span_end;
                    kept.push(finding);
                }
            }
            findings.extend(kept.clone());
            Value::String(apply_redactions(&s, &kept))
        }
        Value::Array(a) => Value::Array(
            a.into_iter()
                .map(|x| redact_recursive(x, patterns, findings))
                .collect(),
        ),
        Value::Object(o) => Value::Object(
            o.into_iter()
                .map(|(k, v)| (k, redact_recursive(v, patterns, findings)))
                .collect(),
        ),
        other => other,
    }
}

/// Walk a value with a caller-supplied hook on every string leaf.
pub fn walk_strings(event: &Event, mut on_string: impl FnMut(&str)) {
    fn inner(v: &Value, on_string: &mut dyn FnMut(&str)) {
        match v {
            Value::String(s) => on_string(s),
            Value::Array(a) => a.iter().for_each(|x| inner(x, on_string)),
            Value::Object(o) => o.values().for_each(|x| inner(x, on_string)),
            _ => {}
        }
    }
    inner(&event.payload, &mut on_string);
}

/// Convenience: count findings for a given event without mutating.
pub fn count_findings(event: &Event, patterns: &[SecretPattern]) -> usize {
    let mut total = 0usize;
    let mut count_one = |_s: &str| {
        let (_, f) = scan_string(_s, patterns);
        total += f.len();
    };
    walk_strings(event, &mut count_one);
    total
}

pub use runlens_core::privacy::Finding as PrivacyFinding;
pub use runlens_core::privacy::SecretPattern as PrivacyPattern;
