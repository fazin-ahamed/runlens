# Changelog

## 0.1.0 — 2025-03-xx

### Added

- Initial domain model: `Event`, `SessionInfo`, `ProjectInfo`, `SessionState`, `EventSource`, `PrivacyClassification`, `Severity`
- ULID-based `Identifier` generation and parsing
- Canonical byte serialization for hash-chain input
- BLAKE3 hash chain: `seal`, `verify_chain`, `seal_chain`
- Verification error types: `VerifyError`, `ChainError`
- Privacy classification and redaction rules
- Session-to-session comparison / divergence explanation
- Ed25519 signature verification over chain heads
- 38 unit tests with frozen hashes for protocol stability
