$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Target = Join-Path $Root "target\windows-tsf"
$Package = Join-Path $Target "package"

Push-Location $Root
try {
    python Bootstrap\Build\check-architecture.py
    if ($LASTEXITCODE) { throw "Architecture check failed" }

    python Bootstrap\Build\check-windows-tsf.py
    if ($LASTEXITCODE) { throw "Windows TSF contract check failed" }

    cargo test -p inputkey-windows-tsf -p inputkey-windows-tsf-bootstrap
    if ($LASTEXITCODE) { throw "Windows TSF tests failed" }

    cargo build --release --target-dir $Target -p inputkey-windows-tsf-bootstrap --lib --bin InputKeyTSFRegister
    if ($LASTEXITCODE) { throw "Windows TSF release build failed" }

    cargo build --release --target-dir $Target -p inputkey-language-pack-vietnamese -p inputkey-language-pack-french
    if ($LASTEXITCODE) { throw "Language pack release build failed" }

    if (Test-Path $Package) { Remove-Item $Package -Recurse -Force }
    New-Item -ItemType Directory -Force $Package | Out-Null
    Copy-Item (Join-Path $Target "release\InputKeyTSF.dll") $Package
    Copy-Item (Join-Path $Target "release\InputKeyTSFRegister.exe") $Package
    $Languages = Join-Path $Package "languages"
    New-Item -ItemType Directory -Force $Languages | Out-Null
    Copy-Item (Join-Path $Target "release\InputKeyLanguageVietnamese.dll") $Languages
    Copy-Item (Join-Path $Target "release\InputKeyLanguageFrench.dll") $Languages

    Write-Host "Built $Package"
} finally {
    Pop-Location
}
