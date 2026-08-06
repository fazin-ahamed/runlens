# Top-level build script for RunLens (Windows / PowerShell).
# Mirrors the CI pipeline steps.

$ErrorActionPreference = "Stop"
Set-Location -LiteralPath $PSScriptRoot

Write-Host "[1/3] cargo build --workspace"
cargo build --workspace

Write-Host "[2/3] cargo test --workspace"
cargo test --workspace

Write-Host "[3/3] zed extension wasm"
$targets = rustup target list --installed 2>$null
if ($targets -match "wasm32-wasip2") {
    cargo build --release --target wasm32-wasip2 --manifest-path extensions/zed-runlens/Cargo.toml
    Write-Host "  artefact: extensions/zed-runlens/target/wasm32-wasip2/release/zed_runlens_extension.wasm"
} else {
    Write-Host "  wasm32-wasip2 target not installed; skipping wasm build"
}

Write-Host "build.ps1: done"
