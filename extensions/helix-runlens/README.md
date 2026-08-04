# RunLens for Helix

[Helix](https://helix-editor.com) does not support editor plugins. Integration works alongside the editor.

## Usage

### 1. Run the daemon (separate terminal)

```sh
runlens daemon
```

The daemon listens on `ws://127.0.0.1:9876` and records terminal activity, file events, and system calls for the current project.

### 2. Use the helix-runlens helpers

Source the provided helper script in your shell config:

```sh
source /path/to/extensions/helix-runlens/runlens.sh
```

Now inside Helix you can:

| Command | Description |
|---------|-------------|
| `:sh rl status` | Check daemon status |
| `:sh rl record "label"` | Start recording with a label |
| `:sh rl stop` | Stop recording |
| `:sh rl list` | List recent sessions |
| `:sh rl critical <session-id>` | Show critical path |

Helix's `:sh` runs shell commands inline without leaving the editor.

### 3. AI integration via MCP

RunLens exposes an MCP (Model Context Protocol) server for AI assistant access:

```sh
runlens mcp
```

Configure your AI tool (Claude Code, etc.) to connect to this stdio endpoint to query sessions, inspect traces, and analyze recordings from within your editor workflow.

### 4. `languages.toml`: RunLens event files

RunLens writes structured event files (`.jsonl`). To make Helix syntax-highlight these nicely:

```toml
[[language]]
name = "jsonl"
scope = "source.json"
file-types = ["jsonl"]
indent = { tab-width = 2 }
```

Add this to `.helix/languages.toml` in your project root.
