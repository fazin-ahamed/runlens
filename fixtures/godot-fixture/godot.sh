#!/usr/bin/env bash
# Godot fixture: simulated project logs.
# This file is run by the recorder as a stand-in for a real Godot
# project. The script emits a deterministic mix of normal logs, a
# warning with a missing resource, and a fatal script error so the
# signature normaliser can be exercised end-to-end without the editor.
set -uo pipefail
echo "[INFO] Godot Engine 4.2.1 (mock) starting"
sleep 0.05
echo "[WARN] res://missing/foo.png - Resource not found"
sleep 0.05
echo "SCRIPT ERROR: GDScript::reload: Parse Error: Function \"on_click\" not found in base self."
# Print traceback-like lines so FailureSignature's python recogniser has data.
echo "  File \"res://src/main.gd\", line 4, in _ready"
echo "    _ready"
echo "  File \"res://src/main.gd\", line 7, in on_click"
echo "    on_click"
sleep 0.05
echo "[FATAL] Could not load script: res://src/main.gd"
exit 1
