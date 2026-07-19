//! Canonical byte representation for events.
//!
//! RunLens integrity hashes are produced over a deterministic byte form of
//! each event. This module is the single source of truth for that form and
//! for the corresponding test vectors.
//!
//! Rules:
//!
//! - UTF-8 with no trailing whitespace; no BOM.
//! - Object keys are sorted lexicographically (byte-wise).
//! - Numbers use decimal with no exponent; integers have no decimal point;
//!   floats use Rust's `{}` formatting and are explicitly the IEEE-754
//!   representation chosen by the producer. We only allow integers in
//!   canonical events; payloads can use strings if they need a float.
//! - Booleans are the literal `true` / `false`.
//! - Nulls are forbidden in canonical events (use an empty array or absent
//!   key); if a payload slot accepts `Option<T>`, we drop it when None.
//! - Timestamps are RFC3339 with nanosecond precision and `Z` UTC suffix.
//! - Sequences are decimal integers.
//! - Severity and kind are lower-case string identifiers.
//! - Unknown payload keys are preserved verbatim and sorted normally, so the
//!   hash remains stable across version drift.
//!
//! The chain hash EXCLUDES `previous_hash` and `current_hash` fields when
//! computing the inputs, so that the produced hash equals what we then
//! embed in `current_hash`.

use crate::model::Event;
use chrono::{DateTime, TimeZone, Utc};
use indexmap::IndexMap;
use serde::Serialize;
use serde_json::Value;
use std::fmt::Write;

/// Bytes that the chain hashes for a given event.
///
/// Excludes `previous_hash` and `current_hash`. Everything else is included.
pub fn chain_input_bytes(event: &Event) -> Vec<u8> {
    let mut v = serde_json::Map::new();
    v.insert("event_id".into(), Value::String(event.event_id.clone()));
    v.insert("session_id".into(), Value::String(event.session_id.clone()));
    v.insert("project_id".into(), Value::String(event.project_id.clone()));
    v.insert("sequence".into(), Value::Number(event.sequence.into()));
    v.insert("source".into(), Value::String(event.source.as_str().into()));
    v.insert("kind".into(), Value::String(event.kind.as_str().into()));
    v.insert(
        "severity".into(),
        Value::String(event.severity.as_str().into()),
    );
    v.insert(
        "utc_timestamp".into(),
        Value::String(format_rfc3339_nanos(event.utc_timestamp)),
    );
    v.insert(
        "monotonic_ns".into(),
        Value::Number(event.monotonic_ns.into()),
    );
    if let Some(d) = event.duration_ns {
        v.insert("duration_ns".into(), Value::Number(d.into()));
    }
    if let Some(c) = &event.correlation_id {
        v.insert("correlation_id".into(), Value::String(c.clone()));
    }
    if let Some(p) = &event.parent_event_id {
        v.insert("parent_event_id".into(), Value::String(p.clone()));
    }
    v.insert(
        "payload_version".into(),
        Value::Number(event.payload_version.into()),
    );
    v.insert("payload".into(), event.payload.clone());
    v.insert(
        "classification".into(),
        Value::String(event.classification.as_str().into()),
    );
    sort_map_keys(&mut v);
    let bytes = serde_json::to_vec(&Value::Object(v)).expect("canonical serializable");
    bytes
}

/// The full canonical bytes including both `previous_hash` (if known) and
/// the freshly computed `current_hash`. Useful for exports and human
/// inspection; this is NOT what the chain hashes use.
pub fn full_canonical_bytes(event: &Event) -> Vec<u8> {
    let mut v = serde_json::Map::new();
    if let Some(prev) = &event.previous_hash {
        v.insert("previous_hash".into(), Value::String(prev.clone()));
    }
    if let Some(curr) = &event.current_hash {
        v.insert("current_hash".into(), Value::String(curr.clone()));
    }
    let inner: serde_json::Map<String, Value> = serde_json::from_slice(&chain_input_bytes(event))
        .expect("round-trip invariant");
    for (k, vv) in inner {
        v.insert(k, vv);
    }
    sort_map_keys(&mut v);
    serde_json::to_vec(&Value::Object(v)).expect("canonical serializable")
}

/// RFC3339 timestamp with nanosecond precision and explicit `Z`.
pub fn format_rfc3339_nanos(t: DateTime<Utc>) -> String {
    let secs = t.timestamp();
    let nsec = t.timestamp_subsec_nanos();
    let dt = Utc
        .timestamp_opt(secs, nsec)
        .single()
        .unwrap_or_else(Utc::now);
    // Always emit Z for canonical form (collapse TZ if somehow offset).
    let mut s = dt.format("%Y-%m-%dT%H:%M:%S%.9f").to_string();
    s.push('Z');
    s
}

/// Strip field keys we exclude from chain hashing so the canonical map is
/// unambiguous without relying on field ordering.
pub fn sort_map_keys(map: &mut serde_json::Map<String, Value>) {
    let keys: Vec<String> = map.keys().cloned().collect();
    let pairs: Vec<(String, Value)> = keys
        .into_iter()
        .map(|k| {
            let v = map.remove(&k).unwrap();
            (k, v)
        })
        .collect();
    map.clear();
    let mut sorted: Vec<(String, Value)> = pairs.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    for (k, v) in sorted {
        map.insert(k, v);
    }
}

/// Pretty-print a sorted map of values for debugging. NOT canonical.
pub fn debug_canonical(v: &impl Serialize) -> String {
    let mut s = String::new();
    let bytes = serde_json::to_vec(v).expect("debug serializable");
    let value: Value = serde_json::from_slice(&bytes).expect("valid json");
    write_value(&mut s, &value);
    s
}

fn write_value(s: &mut String, v: &Value) {
    match v {
        Value::Null => s.push_str("null"),
        Value::Bool(b) => s.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                write!(s, "{i}").unwrap();
            } else if let Some(u) = n.as_u64() {
                write!(s, "{u}").unwrap();
            } else if let Some(f) = n.as_f64() {
                // Avoid scientific notation for subnormals.
                if f.is_finite() && f.abs() < 1e-3 {
                    write!(s, "{f:.20}").unwrap();
                } else {
                    write!(s, "{f}").unwrap();
                }
            } else {
                s.push_str("0");
            }
        }
        Value::String(st) => {
            s.push('"');
            for ch in st.chars() {
                match ch {
                    '"' => s.push_str("\\\""),
                    '\\' => s.push_str("\\\\"),
                    '\n' => s.push_str("\\n"),
                    '\r' => s.push_str("\\r"),
                    '\t' => s.push_str("\\t"),
                    '\u{08}' => s.push_str("\\b"),
                    '\u{0c}' => s.push_str("\\f"),
                    c if (c as u32) < 0x20 => write!(s, "\\u{:04x}", c as u32).unwrap(),
                    c => s.push(c),
                }
            }
            s.push('"');
        }
        Value::Array(a) => {
            s.push('[');
            for (i, item) in a.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                write_value(s, item);
            }
            s.push(']');
        }
        Value::Object(m) => {
            let mut entries: Vec<(&String, &Value)> = m.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            s.push('{');
            for (i, (k, v)) in entries.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                write_value(s, &Value::String((*k).clone()));
                s.push(':');
                write_value(s, v);
            }
            s.push('}');
        }
    }
}

/// Helper to build a sorted IndexMap. Currently only used by tests but kept
/// here to share with future canonical helpers.
#[allow(dead_code)]
pub fn sorted_map() -> IndexMap<String, Value> {
    IndexMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Event;
    use chrono::TimeZone;

    fn sample_event(seq: u64, ts_secs: i64, nsec: u32) -> Event {
        let mut e = Event::sample_at(seq, ts_secs, nsec);
        e.previous_hash = None;
        e.current_hash = None;
        e
    }

    #[test]
    fn same_inputs_produce_same_bytes() {
        let a = sample_event(7, 1_700_000_000, 123_456_789);
        let b = sample_event(7, 1_700_000_000, 123_456_789);
        assert_eq!(chain_input_bytes(&a), chain_input_bytes(&b));
    }

    #[test]
    fn sequence_changes_bytes() {
        let a = sample_event(7, 1_700_000_000, 0);
        let b = sample_event(8, 1_700_000_000, 0);
        assert_ne!(chain_input_bytes(&a), chain_input_bytes(&b));
    }

    #[test]
    fn timestamp_changes_bytes() {
        let a = sample_event(1, 1_700_000_000, 0);
        let b = sample_event(1, 1_700_000_001, 0);
        assert_ne!(chain_input_bytes(&a), chain_input_bytes(&b));
    }

    #[test]
    fn ts_format_includes_nanoseconds() {
        let t = Utc.timestamp_opt(1_700_000_000, 5).single().unwrap();
        assert_eq!(format_rfc3339_nanos(t), "2023-11-14T22:13:20.000000005Z");
    }

    #[test]
    fn ts_format_zeros_pad_nanoseconds() {
        let t = Utc.timestamp_opt(1, 0).single().unwrap();
        assert_eq!(format_rfc3339_nanos(t), "1970-01-01T00:00:01.000000000Z");
    }

    #[test]
    fn known_vector_for_first_event() {
        let e = sample_event(0, 1_700_000_000, 0);
        let bytes = chain_input_bytes(&e);
        let hash = blake3::hash(&bytes).to_hex().to_string();
        // The vector is locked; changing it means protocol break.
        let expected = sample_event_blueprint_hash();
        assert_eq!(hash, expected, "update locked vector and document the change");
    }

    fn sample_event_blueprint_hash() -> String {
        // Computed at module-defining time; this helper exists so test stays readable.
        crate::model::test_support::BLUEPRINT_HASH_0.to_string()
    }
}
