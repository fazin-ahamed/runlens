# RunLens Vim Plugin

CLI-wrapper plugin for the [RunLens](https://github.com/anomalyco/runlens) developer flight recorder.

## Requirements

- `runlens` binary in `$PATH`

## Install

```vim
set rtp+=path/to/extensions/vim-runlens
```

Or use a plugin manager:

```vim
Plug '~/path/to/extensions/vim-runlens'
```

## Commands

| Command | Description |
|---------|-------------|
| `:RunLensList` | List sessions from `runlens list --json` |
| `:RunLensRecord` | Start a recording |
| `:RunLensStatus` | Check daemon status |

## How it works

This plugin shells out to the `runlens` CLI binary via `system()`. It is intentionally minimal. All protocol logic lives in the binary itself.
