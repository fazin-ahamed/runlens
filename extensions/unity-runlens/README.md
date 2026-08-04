# RunLens Unity Package

A Unity package for integrating the RunLens developer flight recorder into Unity projects for game/XR development recording.

## Files

| File | Purpose |
|------|---------|
| `package.json` | Unity package manifest |
| `Runtime/RunLensClient.cs` | WebSocket JSON-RPC client using `System.Net.WebSockets` |
| `Runtime/RunLensRecorder.cs` | MonoBehaviour for recording scene events, log messages, scene changes |
| `Runtime/RunLensSession.cs` | Session management wrapper (list, start, stop, verify) |
| `Runtime/RunLensSettings.cs` | ScriptableObject for settings |
| `Runtime/DaemonConnection.cs` | Daemon connect/disconnect with auto-reconnect (exponential backoff) |
| `Editor/RunLensEditorWindow.cs` | Editor window for session list and recording control |
| `Editor/RunLensMenuItems.cs` | Menu items (Window > RunLens, Tools > RunLens) |

## Install

1. Copy to `Packages/com.runlens.runlens/` in your Unity project (or use Package Manager > Add package from disk)
2. The editor menu appears under **Window > RunLens > Session Manager** and **Tools > RunLens**

## Usage

- Open **Window > RunLens > Session Manager** to see the editor window
- Click **Start Recording** to begin a session, **Stop Recording** when done
- Sessions are listed with a Verify button
- Attach `RunLensRecorder` to a GameObject for runtime recording (log capture, scene changes)
- Create settings via **Tools > RunLens > Create Settings**

## Requirements

- Unity 2021.3+
- RunLens daemon running (`runlens daemon`)
- .NET 4.x or .NET Standard 2.0 scripting runtime
