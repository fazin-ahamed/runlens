#!/usr/bin/env bash
# Deterministic development fixture: a tiny Rust binary that builds and
# tests in two scenarios that we can record from.
#
#   ./fixtures/dev-fixture/good.sh  -> cargo test always passes
#   ./fixtures/dev-fixture/bad.sh   -> cargo test always fails
#
# Both wrap a small project under ./fixtures/dev-fixture/sample-app so
# tests inside it are reproducible on every host.

set -euo pipefail
cd "$(dirname "$0")/sample-app"
cargo test --quiet --color never
echo "GOOD: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
