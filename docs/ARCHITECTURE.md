# architecture

runlens is 6 rust crates in a cargo workspace. dependency graph is
one-way. core doesnt depend on anything else. everything else depends
on core.

```
record  ─┴────────────┐
list     │            │
show     ▼            ▼
runlens-cli ──► runlens-recorder ──► runlens-core
                        │
                        ▼
                  runlens-storage
                        ▲
                 ┌──────┴──────┐
                 │             │
          runlens-bundle   runlens-mcp
```

## runlens-core

no deps on other runlens crates. contains:

- `model` -- Event, SessionInfo, ProjectInfo. the core data types.
- `identifier` -- ULID wrapper. lowercase, monotonic, in-process gen.
- `canonical` -- deterministic byte encoding for the hash chain.
- `chain` -- BLAKE3 chain. seal events, verify them, detect tampering.
- `privacy` -- regex registry for secret detection. scan, redact, mask.
- `signatures` -- failure signature normalizer. strips noise (UUIDs, PIDs, timestamps).
- `compare` -- explainable diff between sessions. no root cause claims.

## runlens-storage

one sqlite db in WAL mode. schema is versioned with migrations.

- `Repository::open` opens or creates a db, runs migrations.
- migrations live in `src/migrations/0001_initial.sql` (just one for now).
- foreign keys on. integrity-affecting writes are transactional.
- tables: projects, sessions, events, artifacts, event_artifacts, file_states, markers, redaction_findings, imports, comparisons, test_investigations, integrations, bundles.
- `DiskArtifacts` -- on-disk blob store with 2-level fan-out, atomic tmp+rename.

## runlens-recorder

the busy one. orchestrates a recording session.

- `dispatch` -- single-writer channel. assigns sequence + ULID + clock + seal. mutex keeps the hash chain deterministic.
- `redaction` -- wires core::privacy into per-event payload walks.
- `pty` -- cross-platform PTY via portable-pty.
- `file_watcher` -- notify-based recursive watcher with 50ms debounce.
- `git` -- shells out to git for HEAD/branch/dirty/lockfile-hashes.
- `env_fingerprint` -- allow-list only. everything else is Excluded.
- `profiler` -- wall-clock sampler of resource usage.
- `test_adapters` -- parsers for pytest/vitest/junit-xml/go-test output.
- `session` -- Session::record(repo, opts) -> SessionSummary.

## runlens-bundle

handles .runlens export and import.

- `manifest` -- BundleManifest, version compat checking.
- `export` -- verifies chain before writing. writes TOML manifest + JSONL events + artifacts as tar.gz.
- `import` -- path traversal guard + manifest version check + chain re-verify.

## runlens-cli

clap subcommands. each is `pub async fn` that takes WorkspacePaths and returns Result. single multi-threaded runtime built in main.

## runlens-mcp

two transports: stdio (newline-delimited JSON-RPC 2.0) and loopback HTTP (axum). tools: list_sessions, get_session, find_errors, compare_sessions, redactions, verify_session. all read-only.

## perf overview

| layer | target |
|-------|--------|
| core hashes | <5µs per event (blake3) |
| sqlite write | ~1.5ms per event on nvme |
| redaction | bound by regex compilation + O(N) scan |
| file watcher | <5ms p99 from event to emit |
| pty capture | 4KB chunks, drain control |

~2000 events/sec sustained on commodity hardware. more in PERFORMANCE.md.
