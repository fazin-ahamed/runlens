# Top-level build script for RunLens (Windows / PowerShell).
# Mirrors the CI pipeline steps.

$ErrorActionPreference = "Stop"
Set-Location -LiteralPath $PSScriptRoot

Write-Host "[1/3] cargo build --workspace"
cargo build --workspace

Write-Host "[2/3] cargo test --workspace"
cargo test --workspace

Write-Host "[3/3] VS Code extension -> runlens-0.1.0.vsix"
if (Get-Command npm -ErrorAction SilentlyContinue) {
    Push-Location -LiteralPath "extensions/vscode-runlens"
    npm install --no-audit --no-fund --prefer-offline
    if ($LASTEXITCODE -ne 0) { throw "npm install failed" }
    npm run ci
    if ($LASTEXITCODE -ne 0) { throw "npm run ci failed" }
    Pop-Location
    Write-Host ("  artefact: extensions/vscode-runlens/runlens-{0}.vsix" -f "0.1.0")
} else {
    Write-Host "  npm not on PATH; skipping VSIX build"
}

Write-Host "build.ps1: done"
