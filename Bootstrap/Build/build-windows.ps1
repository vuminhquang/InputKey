$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$VersionLine = Select-String -Path (Join-Path $Root "Cargo.toml") -Pattern '^version = "([^"]+)"$' | Select-Object -First 1
if (-not $VersionLine) { throw "Workspace version not found" }
$Version = $VersionLine.Matches[0].Groups[1].Value
$Target = Join-Path $Root ("target\windows-" + $Version)
$Stage = Join-Path $env:TEMP ("InputKey-Windows-" + $Version + "-" + $PID)
$DistDir = Join-Path $Root ("dist\InputKey-Windows-" + $Version)
$Zip = Join-Path $Root ("dist\InputKey-Windows-" + $Version + ".zip")
$DistExe = Join-Path $DistDir "InputKey.exe"
$RestartInputKey = $false

Push-Location $Root
try {
    python Bootstrap\Build\check-architecture.py
    if ($LASTEXITCODE) { throw "Architecture check failed" }

    python Bootstrap\Build\check-windows-hook.py
    if ($LASTEXITCODE) { throw "Windows compatibility check failed" }

    python Bootstrap\Build\check-windows-tsf.py
    if ($LASTEXITCODE) { throw "Windows TSF contract check failed" }

    python Bootstrap\Build\check-windows-startup.py
    if ($LASTEXITCODE) { throw "Windows startup persistence contract check failed" }

    cargo test -p inputkey-windows-settings -p inputkey-windows-clipboard -p inputkey-windows-hook -p inputkey-windows-control -p inputkey-windows-tsf -p inputkey-windows-tsf-bootstrap -p inputkey-windows-control-bootstrap
    if ($LASTEXITCODE) { throw "Windows tests failed" }

    cargo build --release --target-dir $Target -p inputkey-windows-control-bootstrap --bin InputKey --bin InputKeyCompatibility
    if ($LASTEXITCODE) { throw "Windows control release build failed" }

    cargo build --release --target-dir $Target -p inputkey-windows-tsf-bootstrap --lib --bin InputKeyTSFRegister
    if ($LASTEXITCODE) { throw "Windows TSF release build failed" }

    cargo build --release --target-dir $Target -p inputkey-language-pack-vietnamese -p inputkey-language-pack-french -p inputkey-language-pack-danish -p inputkey-language-pack-swedish -p inputkey-language-pack-german
    if ($LASTEXITCODE) { throw "Language pack release build failed" }

    if (Test-Path $Stage) { Remove-Item $Stage -Recurse -Force }
    New-Item -ItemType Directory -Force $Stage | Out-Null
    Copy-Item (Join-Path $Target "release\InputKey.exe") $Stage
    Copy-Item (Join-Path $Target "release\InputKeyCompatibility.exe") $Stage
    Copy-Item (Join-Path $Target "release\InputKeyTSFRegister.exe") $Stage
    Copy-Item "Implementations\WindowsHook\inputkey-v.ico" $Stage
    Copy-Item "Implementations\WindowsHook\inputkey-e.ico" $Stage
    Copy-Item "VERSION" $Stage
    $Runtime = Join-Path $Stage ("runtime\" + $Version)
    New-Item -ItemType Directory -Force $Runtime | Out-Null
    Copy-Item (Join-Path $Target "release\InputKeyTSF.dll") $Runtime
    $Languages = Join-Path $Runtime "languages"
    New-Item -ItemType Directory -Force $Languages | Out-Null
    Copy-Item (Join-Path $Target "release\InputKeyLanguageVietnamese.dll") $Languages
    Copy-Item (Join-Path $Target "release\InputKeyLanguageFrench.dll") $Languages
    Copy-Item (Join-Path $Target "release\InputKeyLanguageDanish.dll") $Languages
    Copy-Item (Join-Path $Target "release\InputKeyLanguageSwedish.dll") $Languages
    Copy-Item (Join-Path $Target "release\InputKeyLanguageGerman.dll") $Languages

    New-Item -ItemType Directory -Force (Split-Path $Zip) | Out-Null
    $RunningDistInputKey = Get-Process InputKey -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $DistExe }
    if ($RunningDistInputKey) {
        $RunningDistInputKey | Stop-Process -Force
        $RestartInputKey = $true
        Start-Sleep -Milliseconds 250
    }
    if (Test-Path $DistDir) {
        $PreviousDist = $DistDir + "-loaded-" + (Get-Date -Format "yyyyMMdd-HHmmss")
        Rename-Item -LiteralPath $DistDir -NewName (Split-Path $PreviousDist -Leaf)
        try {
            Remove-Item $PreviousDist -Recurse -Force -ErrorAction Stop
        } catch {
            Write-Host "Kept loaded previous runtime at $PreviousDist"
        }
    }
    Copy-Item $Stage $DistDir -Recurse -Force
    if (Test-Path $Zip) { Remove-Item $Zip -Force }
    Compress-Archive -Path (Join-Path $Stage "*") -DestinationPath $Zip

    Write-Host "Packaged $Zip"
} finally {
    if ($RestartInputKey -and (Test-Path $DistExe)) {
        $AlreadyRunning = Get-Process InputKey -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $DistExe }
        if (-not $AlreadyRunning) { Start-Process $DistExe }
    }
    if (Test-Path $Stage) { Remove-Item $Stage -Recurse -Force -ErrorAction SilentlyContinue }
    Pop-Location
}
