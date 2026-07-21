# RunLens Godot plugin (scaffold)

This directory contains a Godot 4.x addon scaffold. The plugin does
NOT currently run inside the Godot editor and must be verified by a
human reviewer before publishing. The scaffold registers an
autoload singleton and exposes a function that wraps the runlens CLI.

## Status

- Source-only. No `.cfg` icons shipped.
- `RunLens.gd` is an autoload singleton stub.
- `plugin.cfg` declares the addon metadata.
- Documented limitations: no runtime editor verification on this
  host. The fixture at `fixtures/godot-fixture/godot.sh` is a
  non-editor reproducer that emits logs the recorder already
  understands.

## Install

1. Copy the contents of this directory to `res://addons/runlens/` in
   a Godot 4.2+ project.
2. Enable the plugin via Project > Project Settings > Plugins.
3. The autoload singleton `RunLens` becomes available globally.
