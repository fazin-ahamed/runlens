# RunLens Neovim Plugin

A WebSocket JSON-RPC client for the [RunLens](https://github.com/anomalyco/runlens) developer flight recorder daemon.

## Requirements

- Neovim ≥ 0.9 (for `vim.base64`, `vim.json`, `vim.fn.sockconnect`)
- RunLens daemon running at `ws://localhost:9876`

## Setup

Using lazy.nvim:

```lua
{
  dir = 'path/to/extensions/neovim-runlens',
  config = function()
    require('runlens').setup({ host = 'localhost', port = 9876 })
    require('runlens').commands()
  end,
}
```

Or packer.nvim / paq / manual:

```vim
set rtp+=path/to/extensions/neovim-runlens
:RunLensList
```

## Commands

| Command | Description |
|---------|-------------|
| `:RunLensList` | List sessions via `vim.ui.select` |
| `:RunLensRecord` | Toggle recording on/off |
| `:RunLensSessions` | Session picker showing critical path |
| `:RunLensGraph [trace_id]` | Show critical path for a session |
| `:RunLensStatus` | Show daemon connection status |

## Architecture

- `lua/runlens/daemon.lua`: WebSocket JSON-RPC client (raw TCP via `sockconnect`)
- `lua/runlens/session.lua`: session list/start/stop
- `lua/runlens/recording.lua`: record toggle
- `lua/runlens/graph.lua`: critical path and trace display
