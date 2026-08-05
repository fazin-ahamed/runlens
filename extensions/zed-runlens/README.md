# RunLens for Zed

The Zed extension adds RunLens as a local MCP context server. Zed can ask about recorded sessions without rerunning the command.

## Install for development

1. Build the workspace and the extension from the repository root.

```bash
cargo build --workspace
cargo build --target wasm32-wasip2 --release --manifest-path extensions/zed-runlens/Cargo.toml
```

2. Make sure the `runlens` binary is on your `PATH`.
3. Open Zed, press `ctrl+shift+p` and run `zed: install dev extension`.
4. Select the `extensions/zed-runlens` folder.

The extension starts `runlens mcp` through the local process environment. The MCP server reads the SQLite store in `.runlens/`.

## Manual context server setup

You can add the server directly to your Zed settings instead:

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

Run a recording from a project, then ask the assistant about the session.
