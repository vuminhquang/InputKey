$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Extension = Join-Path $Root "Implementations\Chrome\extension"
$Dist = Join-Path $Root "dist\chrome"
$Stage = Join-Path $Root "dist\.chrome-stage"

Push-Location $Root
try {
    rustup target add wasm32-unknown-unknown
    if ($LASTEXITCODE) { throw "Could not install wasm32-unknown-unknown target" }

    cargo build --release -p inputkey-wasm --target wasm32-unknown-unknown
    if ($LASTEXITCODE) { throw "WASM build failed" }

    Copy-Item "target\wasm32-unknown-unknown\release\inputkey_wasm.wasm" (Join-Path $Extension "inputkey.wasm") -Force

    node (Join-Path $Extension "content_test.cjs")
    if ($LASTEXITCODE) { throw "Chrome extension tests failed" }

    Remove-Item $Stage -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force $Stage, $Dist | Out-Null
    Copy-Item (Join-Path $Extension "*") $Stage -Recurse -Force
    Remove-Item (Join-Path $Stage "content_test.cjs") -Force -ErrorAction SilentlyContinue

    $Version = (Get-Content (Join-Path $Root "VERSION") -Raw).Trim()
    $Zip = Join-Path $Dist "InputKey-$Version.zip"
    $Latest = Join-Path $Dist "InputKey.zip"
    Remove-Item $Zip, $Latest -Force -ErrorAction SilentlyContinue
    Compress-Archive -Path (Join-Path $Stage "*") -DestinationPath $Zip -CompressionLevel Optimal
    Copy-Item $Zip $Latest -Force
    Write-Host "Built $Zip"
} finally {
    Remove-Item $Stage -Recurse -Force -ErrorAction SilentlyContinue
    Pop-Location
}
