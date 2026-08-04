# Build the RunLens VS Code extension for VS Codium.
# The standard build produces a .vsix that Codium installs without changes.

$ErrorActionPreference = "Stop"

$vscodeDir = Join-Path $PSScriptRoot "..\..\vscode-runlens"
$codiumDir = Join-Path $PSScriptRoot ".."
$outVsix   = Join-Path $codiumDir "runlens-0.1.0.vsix"

Write-Host "Building RunLens extension for Codium..." -ForegroundColor Cyan
Write-Host "  source: $vscodeDir"
Write-Host "  output: $outVsix"

# Install deps if needed
if (-not (Test-Path (Join-Path $vscodeDir "node_modules"))) {
    Push-Location $vscodeDir
    try {
        npm install --no-fund --no-audit
    } finally {
        Pop-Location
    }
}

# Build + package (copies .vsix to codium dir)
Push-Location $vscodeDir
try {
    npm run ci
    if ($LASTEXITCODE -ne 0) { throw "Build failed" }

    # Copy the .vsix to the codium directory
    $builtVsix = Join-Path $vscodeDir "runlens-0.1.0.vsix"
    if (Test-Path $builtVsix) {
        Copy-Item $builtVsix $outVsix -Force
        Write-Host "Copied .vsix to $outVsix" -ForegroundColor Green
    }
} finally {
    Pop-Location
}

Write-Host "Done. Install with: codium --install-version `"$outVsix`"" -ForegroundColor Green
