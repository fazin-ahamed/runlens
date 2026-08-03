# RunLens for Emacs

Integrates the [RunLens](https://runlens.dev) developer flight recorder into Emacs, using the daemon's WebSocket API (JSON-RPC 2.0) with CLI fallback.

## Requirements

- Emacs 27+ (native `json`, `tabulated-list`, `transient`)
- `websocket.el`: optional, for daemon connectivity (install from ELPA: `M-x package-install RET websocket RET`)
- `runlens` CLI binary on `$PATH`

## Installation

Place `runlens.el` in your `load-path` and require it:

```elisp
(require 'runlens)
(runlens-mode 1)
```

With `use-package`:

```elisp
(use-package runlens
  :ensure t
  :config (runlens-mode 1))
```

## Usage

| Command | Keybinding | Description |
|---------|-----------|-------------|
| `M-x runlens-list` | `C-c r l` | List recorded sessions |
| `M-x runlens-record` | `C-c r r` | Toggle recording |
| `M-x runlens-critical-path` | `C-c r c` | Show critical path for a session |
| `M-x runlens-status` | `C-c r s` | Show daemon connection status |
| `M-x runlens-transient` | | Open transient command menu |

### Session list buffer keybindings

| Key | Description |
|-----|-------------|
| `g` | Refresh |
| `C` | Show critical path for session at point |
| `q` | Quit |

## Configuration

```elisp
(setq runlens-daemon-host "127.0.0.1"
      runlens-daemon-port 9876
      runlens-binary "runlens")
```

## How it works

When `runlens-mode` is active, Emacs connects to the daemon at `ws://127.0.0.1:9876` using `websocket.el`. If the daemon is not running or `websocket.el` is unavailable, all commands fall back to the `runlens` CLI binary automatically.

Start the daemon in a terminal:

```sh
runlens daemon
```
