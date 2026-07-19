# runlens-core

Pure domain types, canonical serialization, BLAKE3 hash chain, failure signatures, privacy classification, and session comparison logic.

**Zero I/O.** This crate has no database, filesystem, or network dependencies — all adapters live in sibling crates.

## Modules

| Module | Purpose |
|--------|---------|
| `model` | Event, SessionInfo, ProjectInfo — the canonical data model |
| `identifier` | ULID-based identifier generation and parsing |
| `canonical` | Deterministic byte serialization for hash-chain input |
| `chain` | BLAKE3 hash chain: seal, verify, error types |
| `privacy` | Privacy classification and payload redaction rules |
| `compare` | Session-to-session diff / divergence explanation |
| `signatures` | Ed25519 session signatures over chain heads |

## Stability

This crate provides the protocol-level types. Changes that alter `chain_input_bytes` output are breaking and require a format version bump.
