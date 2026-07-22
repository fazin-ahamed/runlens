# runlens-storage

SQLite storage layer, schema migrations, and content-addressed blob store for RunLens sessions.

## Modules

| Module | Purpose |
|--------|---------|
| `repo` | `Repository` — typed queries over sessions, events, redactions, comparisons, imports |
| `stores` | `DiskArtifacts` — content-addressed blob storage with integrity verification |
| `migrations` | SQLite schema migrations (current: `0001_initial.sql`) |
