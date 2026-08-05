# architecture

runlens is 13 rust crates in one cargo workspace (`[workspace] members` in
the root `Cargo.toml`). the dependency graph is one-way. `runlens-core`
depends on nothing in the workspace. every other crate depends (directly or
transitively) on core.

workspace members are:

```
runlens-cli          -> core, storage, integrity, recorder, bundle, graph,
                        query + the aux crates below (db, regression, bisect,
                        workspace, proxy, doctor, provenance, ci, minimize,
                        diagnosis)
runlens-daemon       -> core, storage, graph, query
runlens-mcp          -> core, storage, bundle
runlens-recorder     -> core, storage
runlens-bundle       -> core, storage
runlens-graph        -> core, storage
runlens-query        -> core, storage
runlens-rolling      -> core, storage
runlens-storage      -> core
runlens-integrity    -> core
runlens-privacy      -> core
runlens-analysis     -> core
runlens-core         -> (nothing)
```

several crates used by the cli (`runlens-db`, `runlens-regression`,
`runlens-bisect`, `runlens-workspace`, `runlens-proxy`, `runlens-doctor`,
`runlens-provenance`, `runlens-ci`, `runlens-minimize`,
`runlens-diagnosis`) are path dependencies rather than workspace members.
they compile under the same workspace but are not part of `cargo build
--workspace`. treat them as internal helpers with a cli-facing contract.

## runlens-core

no deps on other runlens crates. contains:

- `model`: Event, SessionInfo, ProjectInfo. the core data types.
- `event_v2`: EventV2, the wire model the daemon streams (spans, source
  clock). distinct from the v1 storage Event; convert with `EventV2::from_v1`.
- `identifier`: ULID wrapper. lowercase, monotonic, in-process gen.
- `canonical`: deterministic byte encoding for the hash chain.
- `chain`: BLAKE3 chain. seal events, verify them, detect tampering.
- `privacy`: regex registry for secret detection. scan, redact, mask.
- `signatures`: failure signature normalizer. strips noise (UUIDs, PIDs and timestamps).
- `compare`: explainable diff between sessions. no root cause claims.

## runlens-storage

one sqlite db in WAL mode. schema is versioned with migrations.

- `Repository::open` opens or creates a db, runs migrations.
- migrations live in `src/migrations/0001_initial.sql` (just one for now).
- foreign keys on. integrity-affecting writes are transactional.
- tables: projects, sessions, events, artifacts, event_artifacts, file_states, markers, redaction_findings, imports, comparisons, test_investigations, integrations, bundles.
- `DiskArtifacts`: on-disk blob store with 2-level fan-out and atomic tmp+rename.

## runlens-recorder

the busy one. orchestrates a recording session.

- `dispatch`: single-writer channel. assigns sequence + ULID + clock + seal. mutex keeps the hash chain deterministic. honours a max-event cap.
- `redaction`: wires core::privacy into per-event payload walks. `runlens-confidential` payload classification.
- `pseudotty`: cross-platform pseudo terminal via portable-pty.
- `file_watcher`: notify-based recursive watcher with persistent debounce state; a thread drains its own dispatcher.
- `git`: shells out to git for HEAD, branch, dirty state and lockfile hashes.
- `env_fingerprint`: allow-list only. everything else is Excluded.
- `profiler`: wall-clock sampler of resource usage.
- `test_adapters`: parsers for pytest, vitest, junit-xml and go-test output.
- `session`: Session::record(repo, opts) -> SessionSummary. session.stopped is emitted before the final count is written so the summary reflects the sealed stream.

## runlens-integrity

file-set fingerprints. `composite_hash` folds per-file BLAKE3 hashes sorted by
path so ordering never matters.

## runlens-privacy

sensitive-payload scrubber for the daemon. `EventRecord` marks a payload
classification and drops the value when it is not safe to store.

## runlens-bundle

handles .runlens export and import.

- `manifest`: BundleManifest, version compat checking.
- `export`: verifies chain before writing. writes TOML manifest + JSON events + artifacts as tar.gz.
- `import`: path traversal guard + manifest version check + chain re-verify.

## runlens-rolling

retention policy over stored events and artifacts. deletes old
session-related rows and blob files under a size/age budget.

## runlens-analysis

DB-adjacent analysis helpers (`report`). used by `runlens-db`'s detector
pipeline.

## runlens-graph

graph modeling: `graph`/`critical`/`span`/`diff`. builds event graphs,
critical paths, span chains and session diffs for runlens-query and
runlens-daemon.

## runlens-query

a query language for events. `lexer` -> `parser` -> `ast` -> `executor`,
selections and projections resolved against `runlens-storage`.

## runlens-daemon

the long-running entrypoint. `ws` streams the live pipeline,
`discovery`+`ipc` find and talk to the recorder, `state`/`subscription`
push the stream to consumers. holds the tab replicas that make the v2 wire
model observable.

## runlens-mcp

two transports: stdio (newline-delimited JSON value) and loopback HTTP
(axum). tools: list_sessions, get_session, find_errors, compare_sessions,
redactions, verify_session. all read-only.

## runlens-cli

clap subcommands. each is `pub async fn` that takes WorkspacePaths and returns
Result. single multi-threaded runtime built in main. subcommands include
`record`, `list`, `show`, `verify`, `redactions`, `export`, `import`,
`compare`, `mcp`, `daemon` and the aux crates' commands.

## rust toolchain

rust-version = 1.78, edition 2021. the mcp / zed surface also builds on the
`wasm32-wasip2` target.

## perf overview

approximate, measured on one dev host (see PERFORMANCE.md for hardware and
caveats). treat as order-of-magnitude, not a benchmark guarantee.

| layer | rough cost |
|-------|-----------|
| core hashes | ~1µs per small event (blake3) |
| sqlite write | ~1.5ms per event on nvme |
| redaction | bound by regex scan of payload strings |
| file watcher | <5ms p99 from event to emit |
| pseudo terminal capture | 4KB chunks, drain control |

sustained ~2000 events/sec on commodity hardware in `--release`. more in
PERFORMANCE.md.