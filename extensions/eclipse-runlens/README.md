# RunLens for Eclipse

Eclipse plugin that connects to the RunLens daemon (`ws://localhost:9876`) via JSON-RPC 2.0 WebSocket.

## Prerequisites

- Eclipse IDE for Java Developers (2023-12 or later)
- RunLens daemon running (`runlens daemon`)

## Build

    cd extensions/eclipse-runlens
    # Export as deployable plugin: File → Export → Plug-in Development → Deployable plug-ins and fragments

## Install

Copy the built JAR to `dropins/` or use **Help → Install New Software → Add → Local**.

## Usage

- **Session View**: Window → Show View → Other → RunLens → RunLens Sessions
- **Toggle Recording**: Ctrl+Shift+R (or Window → RunLens → Toggle Recording)
- **Show Critical Path**: Ctrl+Shift+G (or Window → RunLens → Show Critical Path)

## Views

| View             | ID                              | Description                         |
|------------------|---------------------------------|-------------------------------------|
| RunLens Sessions | `runlenseclipse.views.SessionView` | Lists sessions from the daemon    |
