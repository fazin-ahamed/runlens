# RunLens for Sublime Text

Integrates the [RunLens](https://runlens.dev) developer flight recorder into Sublime Text 4.

## Requirements

- Sublime Text 4 (Build 4126+)
- Python 3.8+
- `runlens` CLI binary on `$PATH`
- `websocket-client`: optional, for daemon connectivity (`pip install websocket-client`)

## Installation

Clone this repo and symlink into your Sublime Text `Packages` directory:

```sh
ln -s "$PWD" ~/Library/Application\ Support/Sublime\ Text/Packages/RunLens
```

Or install via Package Control (once published).

## Usage

Commands available from the **Tools → RunLens** menu, or via the command palette (`Ctrl+Shift+P`):

| Command | Description |
|---------|-------------|
| `RunLens: List Sessions` | Show sessions in quick panel |
| `RunLens: Start Recording` | Begin a recording session |
| `RunLens: Show Critical Path` | Pick a session, view critical path in a new buffer |
| `RunLens: Daemon Status` | Check daemon connection |

## Configuration

Create `RunLens.sublime-settings` in your `Packages/User/` directory:

```json
{
    "runlens_binary": "runlens"
}
```

## How it works

The plugin prefers connecting to the daemon via WebSocket (`ws://127.0.0.1:9876`) using the `websocket` Python package. If that's unavailable or the daemon isn't running, it transparently falls back to calling the `runlens` CLI binary.

Start the daemon in a terminal:

```sh
runlens daemon
```
