# RunLens - Android Studio

Android Studio is built on the IntelliJ Platform. The [JetBrains plugin](../jetbrains-runlens/) works directly in Android Studio.

## Setup

1. Build the JetBrains plugin:

       cd extensions/jetbrains-runlens
       ./gradlew build

2. Install in Android Studio: **File → Settings → Plugins → Install Plugin from Disk**
3. Select `jetbrains-runlens/build/libs/runlens-0.1.0.zip`

## Usage

- **RunLens Tool Window**: View → Tool Windows → RunLens
- **Toggle Recording**: Ctrl+Shift+R
- **Show Critical Path**: Ctrl+Shift+G
- **List Sessions**: Ctrl+Shift+L

See the [JetBrains plugin README](../jetbrains-runlens/README.md) for full documentation.
