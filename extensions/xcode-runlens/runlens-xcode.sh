#!/bin/bash
# RunLens Xcode behavior hook
#
# Install: Xcode > Behaviors > Edit Behaviors > (select a trigger) > Run script
#   Shell: /bin/bash
#   Script: path/to/runlens-xcode.sh
set -euo pipefail

RUNLENS="${RUNLENS_BIN:-runlens}"
BEHAVIOR="${XcodeBehavior:-manual}"
LABEL="xcode:${BEHAVIOR}:$(date +%s)"

case "$BEHAVIOR" in
    startsBuild|startsRunning)
        "$RUNLENS" record --label "$LABEL" --start
        ;;
    stopsBuild|stopsRunning)
        "$RUNLENS" record --stop
        ;;
    *)
        "$RUNLENS" record --label "$LABEL"
        ;;
esac