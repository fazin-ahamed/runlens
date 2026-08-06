#!/usr/bin/env bash
# Top-level build script for RunLens. Mirrors the CI pipeline for offline runs.
set -euo pipefail
cd "$(dirname "$0")"

echo "[1/3] cargo build --workspace"
cargo build --workspace

echo "[2/3] cargo test --workspace"
cargo test --workspace

echo "[3/3] zed extension wasm"
if rustup target list --installed 2>/dev/null | grep -q wasm32-wasip2; then
    cargo build --release --target wasm32-wasip2 --manifest-path extensions/zed-runlens/Cargo.toml
    echo "  artefact: extensions/zed-runlens/target/wasm32-wasip2/release/zed_runlens_extension.wasm"
else
    echo "  wasm32-wasip2 target not installed; skipping wasm build"
fi

echo "build.sh: done"
