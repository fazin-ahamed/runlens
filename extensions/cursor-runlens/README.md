# RunLens for Cursor

Two ways to use RunLens in Cursor:

## A) MCP server (recommended for AI chat)

Add `runlens` as an MCP tool so Cursor's AI can query recorded sessions, generate reports, and compare runs.

1. Ensure `runlens` is on your `PATH`
2. Add this config to `.cursor/mcp.json` in your project:

```json
{
  "mcpServers": {
    "runlens": {
      "command": "runlens",
      "args": ["mcp"],
      "transport": "stdio",
      "env": {}
    }
  }
}
```

3. Restart Cursor's MCP server. The `runlens` tools appear in AI chat.

**Exposed MCP tools:**

| Tool                       | What it does                             |
|----------------------------|------------------------------------------|
| `runlens_list`             | List recorded sessions                   |
| `runlens_record`           | Start/stop a recording                   |
| `runlens_show`             | Show session details                     |
| `runlens_query`            | Run an RQL query against recorded data   |
| `runlens_compare`          | Diff two sessions                        |
| `runlens_graph`            | Generate a dependency/flow graph         |
| `runlens_verify`           | Validate session integrity               |

## B) VS Code extension

Cursor supports VS Code extensions directly. Install the `.vsix`:

```
code --install-version extensions/vscode-runlens/runlens-0.1.0.vsix
```

Then use the RunLens sidebar, status bar, and command palette.

## Daemon

Start the WebSocket daemon for live session recording alongside Cursor:

```
runlens daemon
```

Connects at `ws://localhost:9876`. Stop with `Ctrl+C`.
