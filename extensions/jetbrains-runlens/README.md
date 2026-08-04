# RunLens for IntelliJ Platform

Connects to the RunLens daemon (`ws://localhost:9876`) via JSON-RPC 2.0 and provides session browsing, recording control, and graph visualization inside IntelliJ IDEA and other JetBrains IDEs.

## Prerequisites

- IntelliJ IDEA 2023.1+ (Community or Ultimate)
- RunLens daemon running (`runlens daemon`)

## Build

    ./gradlew build

## Install

    ./gradlew runIde
    # Or: File → Settings → Plugins → ⚙ → Install Plugin from Disk → build/libs/runlens-0.1.0.zip

## Usage

- **Tool Window**: View → Tool Windows → RunLens (right sidebar). Lists sessions from the daemon.
- **Toggle Recording**: Ctrl+Shift+R starts or stops recording. The status bar shows connection state.
- **Show Critical Path**: Ctrl+Shift+G displays the critical path for the active recording session.
- **Refresh Sessions**: Ctrl+Shift+L refreshes the session list.

## Commands

| Action              | Shortcut        | Description                          |
|---------------------|-----------------|--------------------------------------|
| Toggle Recording    | Ctrl+Shift+R    | Start or stop the daemon recording   |
| Show Critical Path  | Ctrl+Shift+G    | Show critical path for active session|
| List Sessions       | Ctrl+Shift+L    | Refresh the session list in sidebar  |

## Configuration

The daemon URL defaults to `ws://localhost:9876`. No additional configuration required.
