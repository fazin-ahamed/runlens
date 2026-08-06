# RunLens for Nano

[Nano](https://www.nano-editor.org) is a terminal text editor with no plugin system. RunLens integration works through shell helpers in a separate terminal.

## Setup

Source the helper script from your `~/.bashrc` or `~/.zshrc`:

```sh
source /path/to/extensions/nano-runlens/runlens.sh
```

This provides `rl-*` shell functions.

## Workflow

Run the daemon in **one terminal**:

```sh
runlens daemon
```

Edit files with Nano in **another terminal**. Use the shell functions to control recording:

```sh
rl start "fixing bug #42"
# ... edit with nano, reproduce the bug ...
rl stop
rl list
rl critical $(rl list --latest-id)
```

## Available commands

| Function | Description |
|----------|-------------|
| `rl status` | Check daemon status |
| `rl start <label>` | Start recording with label |
| `rl stop` | Stop recording |
| `rl list [--limit N]` | List sessions |
| `rl critical <session-id>` | Show critical path |
| `rl last` | Show critical path for latest session |

## AI integration

Run the MCP server alongside your workflow:

```sh
runlens mcp
```

This exposes session data to any MCP-compatible AI tool (Claude Code, etc.).
