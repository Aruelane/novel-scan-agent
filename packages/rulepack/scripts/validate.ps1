$ErrorActionPreference = 'Stop'

# This script delegates to the authoritative Node/Ajv validator.
# It does NOT maintain an independent validation path.

$packageRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = Split-Path -Parent (Split-Path -Parent $packageRoot)
$nodeRoot = Join-Path $repoRoot '.toolchain\node-v24.18.0-win-x64'

$env:PATH = "$nodeRoot;$env:PATH"

Push-Location $packageRoot
try {
    $result = npm run validate 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Validation failed: $result"
        exit 1
    }

    $result = npm run validate:negative 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Negative validation failed: $result"
        exit 1
    }

    Write-Output $result
    Write-Output "PowerShell validation wrapper: both validate and validate:negative passed."
} finally {
    Pop-Location
}
