# Changelog

## 0.1.0 — 2025-03-xx

### Added

- SQLite `Repository` with WAL mode and foreign keys
- Schema migration framework (`0001_initial.sql` — 14 tables)
- Session CRUD, event append/list, redaction findings, comparison records
- `DiskArtifacts` content-addressed blob store with BLAKE3 verification
- In-memory repository for test use
- 7 unit tests covering core queries
