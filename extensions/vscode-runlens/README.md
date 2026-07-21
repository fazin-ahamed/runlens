# RunLens VS Code extension

VSIX-packaged extension scaffold for the RunLens recorder. The
extension shells out to the `runlens` binary so the heavy lifting
happens in Rust; the editor handles UX, status, and side-bar
navigation.

## Build

```
cd extensions/vscode-runlens
npm install
npm run build      # bundles src/extension.ts -> out/extension.js
npm run package    # produces runlens-0.1.0.vsix
```

`npm run ci` chains typecheck + build + package in one command.

## Install (vsce-less)

```
code --install-extension runlens-0.1.0.vsix
```

`code` lives on PATH after installing VS Code with the shell
command. The CLI flag is published by VS Code 1.85+.

## Commands

The packaged extension registers these commands; each one shells
out to a `runlens` subcommand:

| id                | effect                                            |
|-------------------|---------------------------------------------------|
| runlens.init      | `runlens init` in the current workspace           |
| runlens.record    | `runlens record ...` via Tasks UI                 |
| runlens.list      | `runlens list --json` for tree-view feed           |
| runlens.showActive| `runlens show <session_id>`                       |
| runlens.verify    | `runlens verify <session_id>`                     |
| runlens.compare   | `runlens compare <baseline> <candidate>`          |

Settings are exposed under the `RunLens` configuration section:
`runlens.cliPath`, `runlens.dbPath`, `runlens.defaultCommand`.

## Status

Compiled + VSIX-packaged at version 0.1.0. The bundled JS is
linearised CJS, only `vscode` is treated as an external runtime
import.
