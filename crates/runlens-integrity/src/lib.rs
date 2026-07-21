use runlens_core::canonical::chain_input_bytes;
use runlens_core::chain;
use runlens_core::model::Event;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IntegrityError {
    #[error("hash mismatch: expected {expected}, found {found}")]
    HashMismatch { expected: String, found: String },
}

// BLAKE3 over per-file BLAKE3 hashes, sorted by path so order never matters.
pub fn composite_hash(hashes: &[(&str, &str)]) -> String {
    let mut entries: Vec<_> = hashes.to_vec();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    let mut hasher = blake3::Hasher::new();
    for (path, hash) in &entries {
        hasher.update(path.as_bytes());
        hasher.update(&[0u8]);
        hasher.update(hash.as_bytes());
        hasher.update(&[0u8]);
    }
    hasher.finalize().to_hex().to_string()
}

// Recomputes every hash in the chain and checks it against the stored one.
pub fn verify_chain(events: &[Event]) -> Result<(), IntegrityError> {
    let mut prev = chain::GENESIS_HASH;
    for event in events {
        let computed = chain::compute_hash(event, prev);

        match &event.current_hash {
            Some(h) if h == &computed => {
                prev = h;
            }
            Some(h) => {
                return Err(IntegrityError::HashMismatch {
                    expected: h.clone(),
                    found: computed,
                });
            }
            None => {
                return Err(IntegrityError::HashMismatch {
                    expected: "(none)".into(),
                    found: computed,
                });
            }
        }
    }
    Ok(())
}

// Reference bytes for a minimal event; every platform must hash them alike.
pub fn known_canonical_bytes() -> Vec<u8> {
    let event = minimal_test_event();
    chain_input_bytes(&event)
}

// Cross-platform reference: BLAKE3(canonical_bytes || 0x00 || genesis).
pub fn known_chain_hash() -> &'static str {
    "c6ec96bbe8db6fcb05ef1ca7b24a83f2d34c3844c7544ffc9fa8c1add403d3d6"
}

fn minimal_test_event() -> Event {
    use chrono::TimeZone;
    Event {
        event_id: "01ar0000000000000000000000".into(),
        session_id: "01ar0000000000000000000001".into(),
        project_id: "01ar0000000000000000000002".into(),
        sequence: 1,
        source: runlens_core::model::EventSource::Core,
        kind: "ping".into(),
        severity: runlens_core::model::Severity::Info,
        utc_timestamp: chrono::Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
        monotonic_ns: 0,
        duration_ns: None,
        correlation_id: None,
        parent_event_id: None,
        payload_version: 1,
        payload: serde_json::json!({"msg": "hello"}),
        classification: runlens_core::model::PrivacyClassification::Public,
        previous_hash: None,
        current_hash: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_bytes_are_deterministic() {
        let a = known_canonical_bytes();
        let b = known_canonical_bytes();
        assert_eq!(a, b, "canonical bytes must be deterministic");
    }

    #[test]
    fn known_hash_is_stable() {
        let event = minimal_test_event();
        let hash = chain::compute_hash(&event, chain::GENESIS_HASH);
        assert_eq!(hash, known_chain_hash(), "cross-platform hash mismatch");
    }

    #[test]
    fn composite_hash_is_deterministic() {
        let a = composite_hash(&[("a", "abc"), ("b", "def")]);
        let b = composite_hash(&[("b", "def"), ("a", "abc")]);
        assert_eq!(a, b, "composite hash must be order-independent");
    }

    #[test]
    fn verify_chain_valid() {
        let e = minimal_test_event();
        let mut sealed = e.clone();
        chain::seal(&mut sealed, chain::GENESIS_HASH);
        assert!(verify_chain(&[sealed]).is_ok());
    }

    #[test]
    fn verify_chain_bad_hash() {
        let mut e = minimal_test_event();
        e.current_hash = Some("bad".into());
        e.previous_hash = Some(chain::GENESIS_HASH.into());
        assert!(verify_chain(&[e]).is_err());
    }
}
