# limitations

runlens is v0.1.0. a lot of stuff isnt perfect. heres what I know
about so you dont discover it the hard way.

## cross-platform

- **windows PTY** uses portable-pty's ConPTY backend. sometimes
  ConPTY drops the first byte of stdout on certain terminal widths.
  I force 120 cols to avoid it but your mileage may vary.
- **macOS** PTY works but I dont test it regularly.
- file watcher uses platform backends (FSEvents on mac, inotify on
  linux, ReadDirectoryChangesW on windows). they all behave slightly
  differently.

## recorder

- **PTY signal handling is lossy.** portable-pty's ExitStatus has
  signal() as a private field so I only get exit codes.
- **PTY drain loop sleeps 25ms fixed.** fine for normal use but adds
  latency to session.end.
- **no interactive stdin.** I dont forward terminal input.
  record-only.
- **single-writer mutex around emit.** limits concurrent emitters
  to one. fine in practice since I only have one recorder plus
  occasional profiler writes.

## file watcher

- fixed 50ms debounce. no dynamic adjustment.
- default ignore list is conservative (target/, node_modules/, .git/).
  can be overridden but not enlarged per-project.
- watching 100K files eats noticeable memory. dont watch the whole
  filesystem.

## git

- shells out to `git`. costs 80-100ms on large dirty repos.
- intentionally not using libgit2 (avoids C dep). means no reflog or
  stash capture.

## env fingerprint

- I dont enumerate process env by default. caller passes it in.
- preview-then-hash is a heuristic. long values with special chars
  are hash-only.

## privacy

- patterns are regex-based. cloud-issued rotated tokens may slip
  through.
- no differential privacy layer.
- interactive accept/reject for findings is coming later.

## bundle

- gzip only. no zstd yet.
- no incremental export. full export every time.
- symlinks in archives are read as regular files (conservative path
  guard).
- large payloads inflate the archive (in-memory read). streaming
  import is future work.

## compare / investigate

- `compare_sessions` shows explainable divergences but does not claim
  root cause. it can show: new event kinds, count changes (>10%
  threshold), proximity of first error, distinct error kinds.
- weights are hardcoded. config knob coming.

## MCP

- stdio transport is scaffold-grade. it responds to initialize and
  tools/list but tools/call defers to HTTP. reason: I didnt want a
  half-baked stdio handler dropping errors.
- HTTP transport binds to 127.0.0.1 only. LAN binding intentionally
  not provided. use SSH tunnel if you need it.

## storage

- no encryption at rest.
- WAL mode on. crash recovery is as good as SQLite gets. events
  mid-flight at SIGKILL may be lost.

## testing

- recorder has 12 unit tests but no integration test that spawns a
  real PTY child end-to-end. reason: ConPTY is flaky on the host I
  develop on. I test manually.
- VS Code extension is compiled-only, not packaged.
- Godot plugin is a source scaffold, not editor-verified.

## what I wont change (ever)

- I will never call the network during recording.
- I will never publish telemetry.
- I will never silently drop redaction findings. every finding is
  available via `runlens redactions`.

