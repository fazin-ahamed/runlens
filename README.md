# runlens

runlens is a local-first flight recorder for your dev machine. when you run a command it watches what happens. terminal output, file changes, git state, env, all of it. then it seals the whole thing into a blake3 hash chain you can go back to later when something breaks.

no internet is used and no telemetry is sent. everything stays local in a sqlite database under `.runlens/`.

## features

- blake3 chain. events get hashed in order. `verify` catches tampering.
- private by default. secrets in terminal output are redacted before they hit storage.
- portable bundles. export a session to a `.runlens` file, share it, verify on import.
- session comparison. see where two runs went different. it shows you the divergence, not a root cause.
- query and analysis. filter and graph what happened instead of paging through raw events.
- mcp integration. claude code, cursor, continue.dev and zed can read a recording without rerunning anything.

## feature status

honest status of the bigger surfaces. v0.1.0, so expect rough edges.

| area | status |
|------|--------|
| recording (pty, file watch, git, env, profiler) | working, tested per-crate |
| redaction + findings | working; regex best-effort (see [docs/PRIVACY](docs/PRIVACY.md)) |
| hash chain seal + verify | working |
| export / import bundles | working, gzip only |
| query (rql) | working core, small grammar |
| graph / critical path / diff | working |
| daemon + live stream | working |
| mcp (stdio + loopback http) | working, read-only |
| git bisect | working against a real worktree |
| editor extensions | mostly scaffolds, one file each |

## how it works

`runlens record` wraps a command in a pseudo terminal. while it runs, collectors pick up output, file writes, git snapshots and env fingerprints. dispatchers route each event through the redaction pipeline and then into the hash chain. the whole stream ends up in one sqlite database under `.runlens/`.

## install

prebuilt binaries for windows, linux and macos are attached to each [github release](https://github.com/fazin-ahamed/runlens/releases). download the tarball for your platform, unpack it, and put `runlens` on your `PATH`.

to build from source you need a rust toolchain with the `wasm32-wasip2` target. the mcp server and the zed extension both build on it.

```bash
cargo build --workspace
target/debug/runlens init
```

or `cargo build --release` if you want a fast binary for daily use.

## record a session

Point `runlens record` at a command. everything after the `--` is what actually runs.

```bash
runlens record . -- cargo test
runlens record . -- pnpm test --run
runlens record . -- node --test
```

then:

```bash
runlens list                # recent sessions, newest first
runlens show <session>      # the full event stream
runlens verify <session>    # check the hash chain
runlens redactions          # what got caught and masked
```

## compare two sessions

```bash
runlens show <first_session>
runlens show <second_session>
```

or pull a session out as a portable file:

```bash
runlens export <session> -o session.runlens
runlens import session.runlens
```

## zed extension

runlens ships as a zed extension. it wires up an mcp context server so the assistant can read what happened in a session. the source lives at `extensions/zed-runlens/`.

record from inside zed, then ask the assistant about what ran.

https://raw.githubusercontent.com/fazin-ahamed/runlens/main/docs/assets/zed-demo.mp4

install it as a dev extension:

1. open zed, hit `ctrl+shift+p`
2. run `zed: install dev extension`
3. pick the `extensions/zed-runlens` folder

the extension spawns `runlens mcp` on your `PATH`, so make sure the runlens binary is on `PATH` before you start. rebuild the wasm with:

```bash
cargo build --target wasm32-wasip2 --release --manifest-path extensions/zed-runlens/Cargo.toml
```

if you would rather wire it by hand, add this to your editor settings:

```json
{
  "context_servers": {
    "runlens": {
      "command": "runlens",
      "args": ["mcp"],
      "transport": "stdio",
      "env": {}
    }
  }
}
```

## editors and engines

| target | what it does |
|--------|--------------|
| [zed](extensions/zed-runlens/README.md) | mcp context server and dev extension |
| [android studio](extensions/android-studio-runlens/README.md) | android studio integration |
| [cursor](extensions/cursor-runlens/README.md) | mcp configuration |
| [eclipse](extensions/eclipse-runlens/README.md) | eclipse plugin scaffold |
| [emacs](extensions/emacs-runlens/README.md) | emacs integration |
| [godot](extensions/godot-runlens/README.md) | godot project integration |
| [helix](extensions/helix-runlens/README.md) | shell and mcp integration |
| [jetbrains](extensions/jetbrains-runlens/README.md) | jetbrains plugin scaffold |
| [neovim](extensions/neovim-runlens/README.md) | neovim plugin |
| [nano](extensions/nano-runlens/README.md) | nano shell helper |
| [sublime](extensions/sublime-runlens/README.md) | sublime integration |
| [unity](extensions/unity-runlens/README.md) | unity project integration |
| [vim](extensions/vim-runlens/README.md) | vim plugin |
| [vscodium](extensions/vscodium-runlens/README.md) | vscodium extension notes |
| [vs code](extensions/vscode-runlens/README.md) | vscode extension scaffold |
| [windsurf](extensions/windsurf-runlens/README.md) | mcp configuration |
| [xcode](extensions/xcode-runlens/README.md) | xcode integration |

the `extensions/` folder holds the scaffolds. most are one file each, a starting point more than a finished plugin.

## docs

- [architecture](docs/ARCHITECTURE.md), how the crates fit together
- [privacy](docs/PRIVACY.md), what gets captured and what does not
- [performance](docs/PERFORMANCE.md), the overhead numbers
- [threat model](docs/THREAT-MODEL.md), security analysis
- [limitations](docs/LIMITATIONS.md), known gaps

## layout

```
crates/
  runlens-core       domain types, chain, canonical bytes
  runlens-storage    sqlite plus blobs
  runlens-recorder   pseudo terminal, file watcher, git, env, session
  runlens-bundle     the .runlens archive format
  runlens-privacy    redaction pipeline
  runlens-integrity  chain verification
  runlens-analysis   sql-aware helpers
  runlens-graph      event graph, critical path, diff
  runlens-query      rql, a small query language
  runlens-rolling    retention policy over stored events
  runlens-mcp        the mcp server binary
  runlens-daemon     the background process, ipc
  runlens-cli        the top-level command line
extensions/          zed, editors, engine fixtures
docs/                 architecture, privacy, performance
```

## ai assistance

this repo uses a bit of ai for learning how to implement the editor
integrations and to navigate the problems faced while attempting to implement
them.

## license

mit, for now. see [LICENSE](LICENSE).

if runlens helps you chase down a bug or pass a test you got stuck on, starring it and using it is the whole thanks. that all.

## contributors

None :(

hopefully no need unless there is an issue, then a pr is more than welcome.
