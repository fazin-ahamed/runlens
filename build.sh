#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

echo "[1/3] cargo build --workspace"
cargo build --workspace

echo "[2/3] cargo test --workspace"
cargo test --workspace

echo "[3/3] VS Code extension -> runlens-0.1.0.vsix"
if command -v npm >/dev/null 2>&1; then
    pushd extensions/vscode-runlens >/dev/null
    npm install --no-audit --no-fund --prefer-offline
    npm run ci
    popd >/dev/null
    echo "  artefact: extensions/vscode-runlens/runlens-0.1.0.vsix"
else
    echo "  npm not on PATH; skipping VSIX build"
fi

echo "build.sh: done"
