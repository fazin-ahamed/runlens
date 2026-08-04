# RunLens Xcode Extension

Xcode Source Editor Extension for the RunLens developer flight recorder.

## Files

| File | Purpose |
|------|---------|
| `RunLensXcode/RunLensXcodeExtension.swift` | Xcode extension handler with Record/List commands |
| `RunLensXcode/DaemonClient.swift` | WebSocket JSON-RPC client using `URLSessionWebSocketTask` |
| `RunLensXcode/Info.plist` | Extension configuration with command definitions |
| `runlens-xcode.sh` | Shell script for Xcode behavior integration (build/run triggers) |

## Install (Extension)

Xcode Source Editor Extensions are limited: no persistent UI, no background connections, each command runs as a separate process.

1. Open `RunLensXcode.xcodeproj` in Xcode (see below to create it)
2. Select the **RunLensXcode** target
3. Build and run. Xcode will prompt to enable the extension in System Preferences.
4. Enable under **System Preferences > Extensions > Xcode Source Editor**
5. Commands appear in **Editor > RunLens > Record Session / List Sessions**

### Create the Xcode project

```bash
# In Xcode:
# 1. File > New > Project > macOS > Xcode Source Editor Extension
# 2. Replace the generated sources with files from RunLensXcode/
# 3. Update Info.plist with the provided one (contains command definitions)
# 4. Remove the default command class, add RecordSessionCommand and ListSessionsCommand
```

## Install (behavior script alternative)

Xcode behaviors are more practical for continuous recording:

1. **Xcode > Behaviors > Edit Behaviors**
2. Select a trigger (e.g., "Starts Running", "Stops Running")
3. Check **Run script** and point to `runlens-xcode.sh`
4. The script calls `runlens record` with the behavior as the label

```bash
# Test the script directly:
RUNLENS_BIN=/usr/local/bin/runlens XcodeBehavior=startsRunning ./runlens-xcode.sh
```

## Commands

| Command | Identifier | Effect |
|---------|-----------|--------|
| Record Session | `com.runlens.xcode.recordSession` | Start/stop daemon recording, CLI fallback |
| List Sessions | `com.runlens.xcode.listSessions` | Show recent sessions via NSAlert, CLI fallback |

## Limitations

- Xcode Source Editor Extensions cannot maintain persistent WebSocket connections
- Each command opens a new connection with a 1-second timeout (then falls back to CLI)
- No persistent UI. Results are shown via NSAlert.
- For continuous recording, use the Xcode behavior script approach
