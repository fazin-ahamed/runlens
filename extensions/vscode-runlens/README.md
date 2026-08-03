# RunLens for VS Code

This directory contains a VS Code extension scaffold. It is not a finished extension and it does not ship a VSIX yet.

The current stub registers these command names:

- `runlens.init`
- `runlens.record`
- `runlens.list`
- `runlens.showActive`
- `runlens.verify`
- `runlens.compare`

Each command currently shows a `not implemented in stub` message. The directory has the generated JavaScript map, the package lock and the test folder, but it does not have a package manifest or a packaged extension.

For a working editor setup, use the [VS Codium guide](../vscodium-runlens/README.md) when its packaging script fits your setup, or connect an editor to the local MCP server directly:

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

The MCP server reads the SQLite store under `.runlens/` on the local machine. Recordings are not sent to a network service.
