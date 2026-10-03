param(
    [Parameter(Mandatory = $true)][string]$Package,
    [Parameter(Mandatory = $true)][string]$Destination
)

$ErrorActionPreference = "Stop"
$Package = (Resolve-Path $Package).Path
$Destination = [IO.Path]::GetFullPath($Destination)
$Stage = Join-Path $env:TEMP ("InputKey-Deploy-" + $PID)
$ClsidKey = "HKCU:\Software\Classes\CLSID\{5F4A4C92-85B3-4F69-A6BC-B427D51D5E50}\InprocServer32"

function Get-TreeSignature([string]$Root) {
    $prefix = [IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
    return @(Get-ChildItem -LiteralPath $Root -Recurse -File | Sort-Object FullName | ForEach-Object {
        [PSCustomObject]@{
            Relative = $_.FullName.Substring($prefix.Length)
            Hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
        }
    })
}

function Assert-SameTree([string]$Left, [string]$Right) {
    $a = @(Get-TreeSignature $Left)
    $b = @(Get-TreeSignature $Right)
    if ($a.Count -ne $b.Count) { return $false }
    for ($i = 0; $i -lt $a.Count; $i++) {
        if ($a[$i].Relative -ne $b[$i].Relative -or $a[$i].Hash -ne $b[$i].Hash) { return $false }
    }
    return $true
}

if (Test-Path $Stage) { Remove-Item $Stage -Recurse -Force }
New-Item -ItemType Directory -Force $Stage | Out-Null

try {
    Expand-Archive -LiteralPath $Package -DestinationPath $Stage -Force
    $VersionFile = Join-Path $Stage "VERSION"
    if (-not (Test-Path $VersionFile)) { throw "Package is missing VERSION" }
    $Version = (Get-Content $VersionFile -Raw).Trim()
    if (-not $Version) { throw "Package VERSION is empty" }

    $RuntimeSource = Join-Path $Stage ("runtime\" + $Version)
    $Required = @(
        (Join-Path $Stage "InputKey.exe"),
        (Join-Path $Stage "InputKeyStartup.exe"),
        (Join-Path $Stage "InputKeyCompatibility.exe"),
        (Join-Path $Stage "InputKeyTSFRegister.exe"),
        (Join-Path $RuntimeSource "InputKeyTSF.dll"),
        (Join-Path $RuntimeSource "languages\InputKeyLanguageVietnamese.dll"),
        (Join-Path $RuntimeSource "languages\InputKeyLanguageFrench.dll")
    )
    foreach ($Path in $Required) { if (-not (Test-Path $Path)) { throw "Package is missing $Path" } }

    New-Item -ItemType Directory -Force $Destination | Out-Null
    $RuntimeRoot = Join-Path $Destination "runtime"
    New-Item -ItemType Directory -Force $RuntimeRoot | Out-Null
    $RuntimeDestination = Join-Path $RuntimeRoot $Version

    if (Test-Path $RuntimeDestination) {
        if (-not (Assert-SameTree $RuntimeSource $RuntimeDestination)) {
            throw "Runtime $Version already exists with different bytes; refusing to replace a versioned runtime that may be loaded."
        }
    } else {
        $Incoming = $RuntimeDestination + ".incoming-" + $PID
        if (Test-Path $Incoming) { Remove-Item $Incoming -Recurse -Force }
        Copy-Item -LiteralPath $RuntimeSource -Destination $Incoming -Recurse
        Move-Item -LiteralPath $Incoming -Destination $RuntimeDestination
    }

    $DestinationExe = Join-Path $Destination "InputKey.exe"

    $DestinationCompatibility = Join-Path $Destination "InputKeyCompatibility.exe"
    foreach ($processName in @("InputKey", "InputKeyCompatibility")) {
        Get-Process $processName -ErrorAction SilentlyContinue | ForEach-Object {
            try {
                $expected = if ($processName -eq "InputKey") { $DestinationExe } else { $DestinationCompatibility }
                if ($_.Path -and ([IO.Path]::GetFullPath($_.Path) -eq [IO.Path]::GetFullPath($expected))) {
                    Stop-Process -Id $_.Id -Force -ErrorAction Stop
                    $_.WaitForExit()
                }
            } catch {
                throw "Could not stop portable $processName process: $($_.Exception.Message)"
            }
        }
    }

    foreach ($Name in @("InputKey.exe", "InputKeyStartup.exe", "InputKeyCompatibility.exe", "InputKeyTSFRegister.exe", "inputkey-v.ico", "inputkey-e.ico", "VERSION")) {
        $Source = Join-Path $Stage $Name
        if (-not (Test-Path $Source)) { continue }
        $Target = Join-Path $Destination $Name
        $Incoming = $Target + ".incoming-" + $PID
        Copy-Item -LiteralPath $Source -Destination $Incoming -Force
        Move-Item -LiteralPath $Incoming -Destination $Target -Force
    }

    $NewDll = Join-Path $RuntimeDestination "InputKeyTSF.dll"
    $OldBinding = $null
    if (Test-Path $ClsidKey) {
        $OldBinding = (Get-ItemProperty $ClsidKey -ErrorAction SilentlyContinue).'(default)'
    }
    $Registrar = Join-Path $Destination "InputKeyTSFRegister.exe"
    & $Registrar --bind-only --dll $NewDll
    if ($LASTEXITCODE) {
        if ($OldBinding -and (Test-Path $OldBinding)) {
            & $Registrar --bind-only --dll $OldBinding | Out-Null
        }
        throw "Could not switch the TSF binding to InputKey $Version"
    }

    $Bound = (Get-ItemProperty $ClsidKey -ErrorAction Stop).'(default)'
    if ([IO.Path]::GetFullPath($Bound) -ne [IO.Path]::GetFullPath($NewDll)) {
        throw "TSF binding verification failed: '$Bound' != '$NewDll'"
    }

    Start-Process -FilePath $DestinationExe | Out-Null
    Write-Host "deployed_version=$Version"
    Write-Host "active_tsf=$NewDll"
    Write-Host "legacy_runtimes=preserved"
} finally {
    if (Test-Path $Stage) { Remove-Item $Stage -Recurse -Force -ErrorAction SilentlyContinue }
}
