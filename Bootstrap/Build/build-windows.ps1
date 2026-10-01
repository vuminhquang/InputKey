$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Dist = Join-Path $Root "dist\windows"

Push-Location $Root
try {
    python Bootstrap\Build\check-architecture.py
    if ($LASTEXITCODE) { throw "Architecture check failed" }

    python Bootstrap\Build\check-windows-hook.py
    if ($LASTEXITCODE) { throw "Windows hook hot-path check failed" }

    cargo test -p inputkey-windows-hook -p inputkey-windows-app
    if ($LASTEXITCODE) { throw "Windows tests failed" }

    cargo build --release -p inputkey-windows-app
    if ($LASTEXITCODE) { throw "Windows release build failed" }

    New-Item -ItemType Directory -Force $Dist | Out-Null
    Copy-Item "target\release\InputKey.exe" (Join-Path $Dist "InputKey.exe") -Force
    Copy-Item "Implementations\WindowsHook\inputkey-v.ico" $Dist -Force
    Copy-Item "Implementations\WindowsHook\inputkey-e.ico" $Dist -Force
    Write-Host "Built $(Join-Path $Dist 'InputKey.exe')"
} finally {
    Pop-Location
}
