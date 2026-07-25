# Changelog

## 0.1.0 — 2025-03-xx

### Added

- `.runlens` archive format: gzip-compressed tar with deterministic layout
- Bundle manifest (`bundle.toml`) with version gating
- Session export: JSONL-written events + artifact blobs
- Safe import: path-traversal prevention, chain re-verification, size validation
- `ImportError` and `ExportError` typed error enums
- 3 unit + 1 integration test
