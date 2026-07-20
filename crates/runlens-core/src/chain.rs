use crate::canonical::{chain_input_bytes, full_canonical_bytes};
use crate::model::Event;
use blake3::Hasher;
use thiserror::Error;

pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

pub fn compute_hash(event: &Event, previous_hash: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(&chain_input_bytes(event));
    hasher.update(&[0u8]);
    hasher.update(previous_hash.as_bytes());
    hasher.finalize().to_hex().to_string()
}

pub fn seal(event: &mut Event, previous_hash: &str) -> String {
    event.previous_hash = Some(previous_hash.to_string());
    let h = compute_hash(event, previous_hash);
    event.current_hash = Some(h.clone());
    h
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    BadGenesis { event_index: u64, found: String },
    BrokenLink { event_index: u64, found: String, expected: String },
    HashMismatch { event_index: u64, found: String, expected: String },
    DuplicateEvent { event_index: u64 },
    SequenceGap { event_index: u64, expected: u64, found: u64 },
    TimeOutOfOrder { event_index: u64, expected: i64, found: i64 },
}

#[derive(Debug, Error)]
pub enum ChainError {
    #[error("chain verification failed at event #{event_index:?}: {reason}")]
    Verification { reason: String, event_index: u64 },
}

pub fn verify_chain(events: &[Event]) -> Result<(), VerifyError> {
    let mut expected_prev = GENESIS_HASH.to_string();
    let mut expected_seq: u64 = 0;
    let mut expected_ts: Option<i64> = None;
    for (i, e) in events.iter().enumerate() {
        let idx = i as u64;
        match &e.previous_hash {
            Some(prev) if prev == &expected_prev => {}
            Some(prev) => {
                return Err(VerifyError::BrokenLink {
                    event_index: idx,
                    found: prev.clone(),
                    expected: expected_prev,
                });
            }
            None => {
                return Err(VerifyError::BadGenesis {
                    event_index: idx,
                    found: String::new(),
                });
            }
        }
        if e.sequence != expected_seq {
            return Err(VerifyError::SequenceGap {
                event_index: idx,
                expected: expected_seq,
                found: e.sequence,
            });
        }
        if let Some(ts) = expected_ts {
            if e.utc_timestamp.timestamp() < ts {
                return Err(VerifyError::TimeOutOfOrder {
                    event_index: idx,
                    expected: ts,
                    found: e.utc_timestamp.timestamp(),
                });
            }
        }
        expected_ts = Some(e.utc_timestamp.timestamp());
        let actual = compute_hash(e, &expected_prev);
        match &e.current_hash {
            Some(curr) if curr == &actual => {}
            Some(curr) => {
                return Err(VerifyError::HashMismatch {
                    event_index: idx,
                    found: curr.clone(),
                    expected: actual,
                });
            }
            None => {
                return Err(VerifyError::HashMismatch {
                    event_index: idx,
                    found: String::new(),
                    expected: actual,
                });
            }
        }
        expected_prev = actual;
        expected_seq += 1;
    }
    Ok(())
}

pub fn seal_chain(events: &mut [Event]) -> String {
    let mut prev = GENESIS_HASH.to_string();
    for e in events.iter_mut() {
        prev = seal(e, &prev);
    }
    prev
}

pub fn dump_canonical(events: &[Event]) -> Vec<u8> {
    let mut out = Vec::new();
    for e in events {
        out.extend_from_slice(&full_canonical_bytes(e));
        out.push(b'\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EventSource, PrivacyClassification, Severity};
    use chrono::{TimeZone, Utc};

    fn make(seq: u64, secs: i64, kind: &str) -> Event {
        let ts = Utc.timestamp_opt(secs, 0).single().unwrap();
        Event {
            event_id: format!("01H{seq:025}"),
            session_id: "01H00000000000000000000001".into(),
            project_id: "01H00000000000000000000002".into(),
            sequence: seq,
            source: EventSource::Core,
            kind: kind.into(),
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
        }
    }

    #[test]
    fn seal_and_verify() {
        let mut chain: Vec<Event> = (0..5).map(|i| make(i, 1_700_000_000i64 + i as i64, "test.sample")).collect();
        let final_hash = seal_chain(&mut chain);
        assert!(!final_hash.is_empty());
        for e in &chain {
            assert!(e.current_hash.is_some());
            assert!(e.previous_hash.is_some());
        }
        assert!(verify_chain(&chain).is_ok());
    }

    #[test]
    fn tamper_after_sealing_is_detected() {
        let mut chain: Vec<Event> = (0..3).map(|i| make(i, 1_700_000_000i64 + i as i64, "test.sample")).collect();
        seal_chain(&mut chain);
        chain[1].payload = serde_json::json!({"sequence": 1, "tampered": true});
        match verify_chain(&chain) {
            Err(VerifyError::HashMismatch { event_index: 1, .. }) => {}
            other => panic!("expected tamper at 1, got {other:?}"),
        }
    }

    #[test]
    fn reorder_is_detected() {
        let mut chain: Vec<Event> = (0..3).map(|i| make(i, 1_700_000_000i64 + i as i64, "test.sample")).collect();
        seal_chain(&mut chain);
        chain.swap(0, 2);
        assert!(matches!(verify_chain(&chain), Err(VerifyError::HashMismatch { .. })
            | Err(VerifyError::BrokenLink { .. })
            | Err(VerifyError::SequenceGap { .. })));
    }

    #[test]
    fn delete_event_is_detected() {
        let mut chain: Vec<Event> = (0..4).map(|i| make(i, 1_700_000_000i64 + i as i64, "test.sample")).collect();
        seal_chain(&mut chain);
        chain.remove(2);
        assert!(verify_chain(&chain).is_err());
    }

    #[test]
    fn insertion_is_detected() {
        let mut chain: Vec<Event> = (0..3).map(|i| make(i, 1_700_000_000i64 + i as i64, "test.sample")).collect();
        seal_chain(&mut chain);
        chain.insert(1, make(99, 1_700_000_500, "test.injected"));
        assert!(verify_chain(&chain).is_err());
    }

    #[test]
    fn duplicate_event_is_detected() {
        let mut chain: Vec<Event> = (0..3).map(|i| make(i, 1_700_000_000i64 + i as i64, "test.sample")).collect();
        seal_chain(&mut chain);
        let dup = chain[1].clone();
        chain.insert(1, dup);
        assert!(verify_chain(&chain).is_err());
    }

    #[test]
    fn time_out_of_order_detected() {
        let mut chain: Vec<Event> = vec![
            make(0, 1_700_000_010, "test.sample"),
            make(1, 1_700_000_009, "test.sample"),
        ];
        seal_chain(&mut chain);
        match verify_chain(&chain) {
            Err(VerifyError::TimeOutOfOrder { .. }) => {}
            other => panic!("expected time-out-of-order, got {other:?}"),
        }
    }

    #[test]
    fn empty_chain_is_ok() {
        assert!(verify_chain(&[]).is_ok());
    }
}
