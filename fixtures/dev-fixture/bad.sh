#!/usr/bin/env bash
# Deterministic development fixture: a tiny Rust binary that we make
# fail in a known way so recorder output is reproducible.

set -uo pipefail
cd "$(dirname "$0")/sample-app"
cargo test --quiet --color never || true
echo "BAD: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "thread 'tests::broken' panicked at 'assertion failed: 1 == 2'" 1>&2
exit 7
