# runlens

runlens is a local-first dev flight recorder. it watches everything
that happens when you run a command — terminal output, file changes,
git state, env, all of it — and seals it into a blake3 hash chain
so you can go back later when stuff breaks.

no network. no telemetry. every byte lives in a sqlite db under `.runlens/`.

## features

- **blake3 chain.** events are hashed in sequence. verify catches tampering.
- **privacy by default.** secrets in terminal output get redacted before storage.
- **portable bundles.** export a session as a `.runlens` file, share it, verify on import.
- **session comparison.** see where two runs diverged (no root cause claims though).
- **mcp integration.** claude code, continue.dev, zed — they can read recordings without re-running.

## quick start

```bash
cargo build --workspace
target/debug/runlens init
target/debug/runlens record -- bash -c "echo hi; exit 0"
target/debug/runlens list
target/debug/runlens verify <session_id>
```

## docs

- `docs/ARCHITECTURE.md` — how the crates fit together
- `docs/PRIVACY.md` — what is and isnt captured
- `docs/THREAT-MODEL.md` — security analysis
- `docs/PERFORMANCE.md` — overhead numbers
- `docs/LIMITATIONS.md` — known gaps

## layout

```
crates/
  runlens-core      domain types, chain, privacy (38 tests)
  runlens-storage   sqlite + blobs (7 tests)
  runlens-recorder  pty, watcher, git, env, session (12 tests)
  runlens-bundle    archive format (4 tests)
  runlens-cli       clap subcommands
  runlens-mcp       stdio + http mcp server
fixtures/
  dev-fixture       working and broken rust projects
  godot-fixture     simulated godot logs
extensions/
  vscode-runlens    typescript extension scaffold
  godot-runlens     gdscript plugin scaffold
  zed-runlens       mcp config
docs/
  architecture, privacy, threat-model, performance, limitations
```

## license

mit or apache-2.0, your pick.
