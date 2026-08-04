# RunLens for VS Codium

VS Codium is a libre/open-source build of VS Code. The existing VS Code extension at `extensions/vscode-runlens/` works on Codium with no code changes.

## Quick install

```
codium --install-version extensions/vscode-runlens/runlens-0.1.0.vsix
```

Or via the UI: Extensions → `...` → Install from VSIX → select `runlens-0.1.0.vsix`.

## Build from source

```powershell
# from the repo root
cd extensions/vscodium-runlens
.\scripts\build-codium.ps1
```

This runs the standard VS Code extension build (typecheck → bundle → package),
producing `runlens-0.1.0.vsix` you can install with `codium --install-version`.

## What's different from VS Code

| Aspect            | VS Code          | VS Codium                    |
|-------------------|------------------|------------------------------|
| Marketplace       | Microsoft        | Open VSX (no Microsoft)      |
| Telemetry         | bundled          | stripped by default          |
| Extension install | `code --install` | `codium --install-version`   |
| Extension host    | same             | same (API-compatible)        |

The RunLens extension has **zero telemetry dependencies**. It does not use `@vscode/extension-telemetry` or App Insights. It works identically on Codium.

## Commands

Same commands as VS Code (registered by the extension):

| id                | effect                                  |
|-------------------|-----------------------------------------|
| `runlens.init`    | `runlens init` in the current workspace |
| `runlens.record`  | `runlens record ...` via Tasks UI       |
| `runlens.list`    | List sessions in tree view              |
| `runlens.showActive` | View session details                |
| `runlens.verify`  | Run RQL query against session           |
| `runlens.compare` | Compare two sessions                    |

## MCP server (alternative)

If you prefer not to install the extension, Codium also supports the MCP protocol
(experimental, via `--mcp` flag). Add the server config to your Codium settings:

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

Requires Codium 1.85+ with MCP support enabled.
