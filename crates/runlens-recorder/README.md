# runlens-recorder

Recording collectors, session orchestration, and redaction pipeline. Spawns a child process under PTY, watches the filesystem, snapshots git/environment state, profiles resources, and feeds structured events into the storage layer.

## Collectors

| Collector | Description |
|-----------|-------------|
| `pty` | Cross-platform PTY child process execution |
| `file_watcher` | Debounced notify-based filesystem change detection |
| `git` | Read-only git HEAD, branch, dirty-path capture |
| `env_fingerprint` | Privacy-safe environment variable snapshot |
| `profiler` | Lightweight wall-clock resource sampling |
| `test_adapters` | JUnit/pytest/vitest/gotest output parsing |
| `redaction` | In-place payload redaction |
| `dispatch` | In-process pub/sub: collectors → storage |
| `session` | Public API for start/stop/lifecycle |
