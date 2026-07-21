# runlens-bundle

Portable `.runlens` archive format -- deterministic gzip-compressed tar with versioned manifest, JSONL event chunks, and content-addressed blobs.

## Format

```
bundle.toml              # versioned manifest
invariants.json          # canonical hash chain hashes
events-0.jsonl           # one JSON Event per line
events-N.jsonl           # continuation chunks (5k events each)
artifacts/<hash>.bin     # content-addressed blobs (2-level fan-out)
```

## Safety

- Path-traversal prevention on import
- Manifest version gating (`COMPATIBLE_VERSIONS`)
- Chain re-verification on import
- Size-mismatch detection per tar entry
