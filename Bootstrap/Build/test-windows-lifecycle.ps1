$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Version = (Get-Content (Join-Path $Root "VERSION") -Raw).Trim()
$PackageRoot = Join-Path $Root "dist\InputKey-Windows-$Version"
$Exe = Join-Path $PackageRoot "InputKey.exe"
$StartupHelper = Join-Path $PackageRoot "InputKeyStartup.exe"
$CompatibilityHelper = Join-Path $PackageRoot "InputKeyCompatibility.exe"
$Registrar = Join-Path $PackageRoot "InputKeyTSFRegister.exe"
$Runtime = Join-Path $PackageRoot ("runtime\" + $Version)
$Dll = Join-Path $Runtime "InputKeyTSF.dll"

foreach ($required in @($Exe, $StartupHelper, $CompatibilityHelper, $Registrar, $Dll)) {
    if (-not (Test-Path -LiteralPath $required)) {
        throw "Missing packaged Windows runtime file: $required"
    }
}
if (Test-Path (Join-Path $PackageRoot "InputKeyTSF.dll")) {
    throw "Side-by-side regression: InputKeyTSF.dll must live under runtime\$Version."
}
if (Test-Path (Join-Path $PackageRoot "languages")) {
    throw "Side-by-side regression: language packs must live with their versioned TSF runtime."
}

$restartPaths = @()
Get-Process InputKey,InputKeyCompatibility -ErrorAction SilentlyContinue | ForEach-Object {
    try {
        if ($_.Path) {
            $restartPaths += $_.Path
            Stop-Process -Id $_.Id -Force -ErrorAction Stop
            $_.WaitForExit()
        }
    } catch {
        throw "Could not stop existing InputKey process: $($_.Exception.Message)"
    }
}
$restartPaths = @($restartPaths | Select-Object -Unique)

$RunPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$runBefore = $null
if (Test-Path $RunPath) {
    $runBefore = (Get-ItemProperty $RunPath -ErrorAction SilentlyContinue).InputKey
}

$StartupShortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Startup\InputKey.lnk"
$StartupBackup = Join-Path $env:TEMP ("InputKey-startup-backup-" + $PID + ".lnk")
$shortcutBeforeExists = Test-Path -LiteralPath $StartupShortcut
$shortcutBeforeHash = $null
if ($shortcutBeforeExists) {
    Copy-Item -LiteralPath $StartupShortcut -Destination $StartupBackup -Force
    $shortcutBeforeHash = (Get-FileHash -LiteralPath $StartupShortcut -Algorithm SHA256).Hash
}

$Inproc = "HKCU:\Software\Classes\CLSID\{5F4A4C92-85B3-4F69-A6BC-B427D51D5E50}\InprocServer32"
function Read-TsfBinding {
    if (-not (Test-Path $Inproc)) { return $null }
    return (Get-ItemProperty $Inproc -ErrorAction SilentlyContinue).'(default)'
}
$bindingBefore = Read-TsfBinding

$SettingsPath = "HKCU:\Software\InputKey\Settings"
$settingsExisted = Test-Path $SettingsPath
$enabledExisted = $false
$originalEnabled = $null
if ($settingsExisted) {
    $props = Get-ItemProperty $SettingsPath -ErrorAction SilentlyContinue
    if ($null -ne $props -and $null -ne $props.Enabled) {
        $enabledExisted = $true
        $originalEnabled = [int]$props.Enabled
    }
}

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class InputKeyWindowProbe {
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr FindWindow(string lpClassName, string lpWindowName);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
}
"@

$process = Start-Process -FilePath $Exe -PassThru
try {
    $window = [IntPtr]::Zero
    $deadline = [DateTime]::UtcNow.AddSeconds(8)
    while ([DateTime]::UtcNow -lt $deadline -and -not $process.HasExited) {
        $window = [InputKeyWindowProbe]::FindWindow("InputKeyControlWindow", "InputKey")
        if ($window -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 50
        $process.Refresh()
    }
    if ($process.HasExited) {
        throw "InputKey exited before its control window became ready (exit code $($process.ExitCode))."
    }
    if ($window -eq [IntPtr]::Zero) {
        throw "InputKeyControlWindow did not appear."
    }

    $compat = $null
    $compatDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([DateTime]::UtcNow -lt $compatDeadline -and $null -eq $compat) {
        $compat = Get-Process InputKeyCompatibility -ErrorAction SilentlyContinue |
            Where-Object { $_.Path -eq $CompatibilityHelper } |
            Select-Object -First 1
        if ($null -eq $compat) { Start-Sleep -Milliseconds 50 }
    }
    if ($null -eq $compat) {
        throw "InputKeyCompatibility did not start beside the control process."
    }

    $runAfter = $null
    if (Test-Path $RunPath) {
        $runAfter = (Get-ItemProperty $RunPath -ErrorAction SilentlyContinue).InputKey
    }
    if ([string]$runAfter -ne [string]$runBefore) {
        throw "Persistence regression: normal InputKey startup changed the legacy Run entry."
    }

    $shortcutAfterExists = Test-Path -LiteralPath $StartupShortcut
    if ($shortcutAfterExists -ne $shortcutBeforeExists) {
        throw "Persistence regression: normal InputKey startup changed the Startup shortcut."
    }
    if ($shortcutBeforeExists) {
        $shortcutAfterHash = (Get-FileHash -LiteralPath $StartupShortcut -Algorithm SHA256).Hash
        if ($shortcutAfterHash -ne $shortcutBeforeHash) {
            throw "Persistence regression: normal InputKey startup rewrote the Startup shortcut."
        }
    }

    $bindingAfter = Read-TsfBinding
    if ([string]$bindingAfter -ne [string]$bindingBefore) {
        throw "TSF startup regression: normal InputKey startup rebound COM from '$bindingBefore' to '$bindingAfter'."
    }

    $enable = Start-Process -FilePath $StartupHelper -ArgumentList @('--enable', '--exe', ('"' + $Exe + '"')) -PassThru -Wait
    if ($enable.ExitCode -ne 0) { throw "InputKeyStartup explicit enable failed." }
    $status = Start-Process -FilePath $StartupHelper -ArgumentList @('--status', '--exe', ('"' + $Exe + '"')) -PassThru -Wait
    if ($status.ExitCode -ne 0) { throw "InputKeyStartup explicit status failed after enable." }
    if (-not (Test-Path -LiteralPath $StartupShortcut)) { throw "InputKeyStartup did not create InputKey.lnk." }
    $runAfterHelper = if (Test-Path $RunPath) { (Get-ItemProperty $RunPath -ErrorAction SilentlyContinue).InputKey } else { $null }
    if ([string]$runAfterHelper -ne [string]$runBefore) {
        throw "Persistence regression: explicit helper wrote the legacy Run entry."
    }
    $disable = Start-Process -FilePath $StartupHelper -ArgumentList @('--disable') -PassThru -Wait
    if ($disable.ExitCode -ne 0) { throw "InputKeyStartup explicit disable failed." }
    if (Test-Path -LiteralPath $StartupShortcut) { throw "InputKeyStartup did not remove InputKey.lnk." }

    if (-not [InputKeyWindowProbe]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
        throw "Could not send WM_CLOSE to InputKeyControlWindow."
    }
    if (-not $process.WaitForExit(8000)) {
        throw "InputKey did not terminate after WM_CLOSE."
    }
    if ($process.ExitCode -ne 0) {
        throw "InputKey exited with code $($process.ExitCode)."
    }

    $compatExitDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while ([DateTime]::UtcNow -lt $compatExitDeadline) {
        $stillRunning = Get-Process InputKeyCompatibility -ErrorAction SilentlyContinue |
            Where-Object { $_.Path -eq $CompatibilityHelper }
        if (-not $stillRunning) { break }
        Start-Sleep -Milliseconds 50
    }
    if (Get-Process InputKeyCompatibility -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -eq $CompatibilityHelper }) {
        throw "InputKeyCompatibility remained running after the control process exited."
    }

    $enabledAfterClose = (Get-ItemProperty $SettingsPath -ErrorAction Stop).Enabled
    if ([int]$enabledAfterClose -ne 0) {
        throw "Graceful close regression: InputKey remained enabled after tray exit."
    }

    Write-Host "windows-lifecycle=ok"
} finally {
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }

    Remove-Item -LiteralPath $StartupShortcut -Force -ErrorAction SilentlyContinue
    if ($shortcutBeforeExists -and (Test-Path -LiteralPath $StartupBackup)) {
        New-Item -ItemType Directory -Force (Split-Path $StartupShortcut) | Out-Null
        Copy-Item -LiteralPath $StartupBackup -Destination $StartupShortcut -Force
    }
    Remove-Item -LiteralPath $StartupBackup -Force -ErrorAction SilentlyContinue

    if ($settingsExisted) {
        if (-not (Test-Path $SettingsPath)) { New-Item -Path $SettingsPath -Force | Out-Null }
        if ($enabledExisted) {
            Set-ItemProperty -Path $SettingsPath -Name Enabled -Type DWord -Value $originalEnabled
        } else {
            Remove-ItemProperty -Path $SettingsPath -Name Enabled -ErrorAction SilentlyContinue
        }
    } else {
        Remove-Item -Path $SettingsPath -Recurse -Force -ErrorAction SilentlyContinue
    }

    foreach ($path in $restartPaths) {
        if (Test-Path -LiteralPath $path) {
            Start-Process -FilePath $path | Out-Null
        }
    }
}
