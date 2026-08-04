# RunLens for Windsurf

Add the `runlens` MCP server so Windsurf's AI (Codeium) can query recorded development sessions.

## Setup

1. Make sure `runlens` is on your `PATH` (install via `cargo install runlens` or download the release binary)
2. Open Windsurf settings and add an MCP server entry:

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

Alternatively, create a `.windsurfrules` file in your project root that includes the MCP config:

```json
{
  "mcp_servers": [
    {
      "name": "runlens",
      "command": "runlens",
      "args": ["mcp"]
    }
  ]
}
```

3. Restart Windsurf. The AI assistant can now call `runlens` tools.

## Exposed MCP tools

| Tool                | Description                           |
|---------------------|---------------------------------------|
| `runlens_list`      | List recorded sessions                |
| `runlens_record`    | Start/stop a recording                |
| `runlens_show`      | View session details                  |
| `runlens_query`     | Query session data with RQL           |
| `runlens_compare`   | Compare two sessions                  |
| `runlens_graph`     | Render a dependency/flow graph        |
| `runlens_verify`    | Check session integrity               |

## Daemon for live recording

Start the daemon alongside Windsurf to capture sessions automatically:

```
runlens daemon
```

The daemon listens on `ws://localhost:9876` using JSON-RPC 2.0. Stop it with `Ctrl+C`.

## Notes

- Windsurf uses the same MCP protocol as Cursor and Zed. The config above works across all three.
- No VS Code extension support. Windsurf runs its own extension host, so use the MCP approach.
- The daemon can run in the background and outlive Windsurf sessions.
