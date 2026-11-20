# Runs Tunelet in development mode.
# Finds the Windows SDK resource compiler (rc.exe) and Cargo even when they
# aren't on PATH, so no system environment changes are needed.
param([string]$Command = "dev")

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path $cargoBin) { $env:Path = "$cargoBin;$env:Path" }

if (-not $env:RC -and -not (Get-Command rc.exe -ErrorAction SilentlyContinue)) {
    $arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
    $rc = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\$arch\rc.exe" -ErrorAction SilentlyContinue |
        Sort-Object FullName -Descending | Select-Object -First 1
    if ($rc) { $env:RC = $rc.FullName } else { Write-Warning "rc.exe not found - install the Windows SDK (see README)." }
}

# Release builds sign update files; use the local key if there is one.
$key = Join-Path $env:USERPROFILE ".tauri\tunelet.key"
if ($Command -eq "build" -and -not $env:TAURI_SIGNING_PRIVATE_KEY -and (Test-Path $key)) {
    $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $key -Raw
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
}

if (-not (Test-Path node_modules)) { npm install }
npm run tauri $Command
