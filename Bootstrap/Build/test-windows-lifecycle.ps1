$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Version = (Get-Content (Join-Path $Root "VERSION") -Raw).Trim()
$PackageRoot = Join-Path $Root "dist\InputKey-Windows-$Version"
$Exe = Join-Path $PackageRoot "InputKey.exe"
$CompatibilityHelper = Join-Path $PackageRoot "InputKeyCompatibility.exe"
$Registrar = Join-Path $PackageRoot "InputKeyTSFRegister.exe"
$Runtime = Join-Path $PackageRoot ("runtime\" + $Version)
$Dll = Join-Path $Runtime "InputKeyTSF.dll"

foreach ($required in @($Exe, $CompatibilityHelper, $Registrar, $Dll)) {
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
$runProps = if (Test-Path $RunPath) { Get-ItemProperty $RunPath -ErrorAction SilentlyContinue } else { $null }
$runBefore = if ($null -ne $runProps) { $runProps.InputKey } else { $null }
$legacyRunBefore = if ($null -ne $runProps) { $runProps.VietnameseKeyboard } else { $null }
if (Test-Path $RunPath) {
    Remove-ItemProperty -Path $RunPath -Name InputKey -ErrorAction SilentlyContinue
    Remove-ItemProperty -Path $RunPath -Name VietnameseKeyboard -ErrorAction SilentlyContinue
}

$StartupShortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Startup\InputKey.lnk"
$StartupBackup = Join-Path $env:TEMP ("InputKey-startup-backup-" + $PID + ".lnk")
$shortcutBeforeExists = Test-Path -LiteralPath $StartupShortcut
if ($shortcutBeforeExists) {
    Copy-Item -LiteralPath $StartupShortcut -Destination $StartupBackup -Force
    Remove-Item -LiteralPath $StartupShortcut -Force
}

$Inproc = "HKCU:\Software\Classes\CLSID\{5F4A4C92-85B3-4F69-A6BC-B427D51D5E50}\InprocServer32"
function Read-TsfBinding {
    if (-not (Test-Path $Inproc)) { return $null }
    return (Get-ItemProperty $Inproc -ErrorAction SilentlyContinue).'(default)'
}
$bindingBefore = Read-TsfBinding
$statusBefore = (& $Registrar --status 2>&1 | Out-String).Trim()
$registeredBefore = $statusBefore -match "registered=true"
$activeBefore = $statusBefore -match "active=true"

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

    $runAfter = if (Test-Path $RunPath) { (Get-ItemProperty $RunPath -ErrorAction SilentlyContinue).InputKey } else { $null }
    if ($null -ne $runAfter) {
        throw "Persistence regression: normal InputKey startup changed the Run entry."
    }
    if (Test-Path -LiteralPath $StartupShortcut) {
        throw "Persistence regression: normal InputKey startup created a Startup shortcut."
    }

    $bindingAfter = Read-TsfBinding
    if ([IO.Path]::GetFullPath([string]$bindingAfter) -ne [IO.Path]::GetFullPath($Dll)) {
        throw "TSF startup regression: InputKey did not repair the current-user COM binding to '$Dll'. Actual: '$bindingAfter'."
    }
    $statusAfter = (& $Registrar --status 2>&1 | Out-String).Trim()
    foreach ($required in @("registered=true", "available=true", "bound=true")) {
        if ($statusAfter -notmatch [regex]::Escape($required)) {
            throw "TSF startup regression: expected '$required' after automatic registration/repair. Status: $statusAfter"
        }
    }
    Write-Host "tsf-status=$statusAfter"

    if (-not [InputKeyWindowProbe]::PostMessage($window, 0x0111, [IntPtr]106, [IntPtr]::Zero)) {
        throw "Could not invoke Start with Windows."
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(3)
    $expectedRun = '"' + $Exe + '"'
    $runEnabled = $null
    while ([DateTime]::UtcNow -lt $deadline) {
        $runEnabled = if (Test-Path $RunPath) { (Get-ItemProperty $RunPath -ErrorAction SilentlyContinue).InputKey } else { $null }
        if ([string]$runEnabled -eq $expectedRun) { break }
        Start-Sleep -Milliseconds 50
    }
    if ([string]$runEnabled -ne $expectedRun) {
        throw "Start with Windows did not write the expected HKCU Run value."
    }
    if (Test-Path -LiteralPath $StartupShortcut) {
        throw "Start with Windows recreated the obsolete Startup shortcut."
    }

    if (-not [InputKeyWindowProbe]::PostMessage($window, 0x0111, [IntPtr]106, [IntPtr]::Zero)) {
        throw "Could not disable Start with Windows."
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(3)
    $runDisabled = $runEnabled
    while ([DateTime]::UtcNow -lt $deadline) {
        $runDisabled = if (Test-Path $RunPath) { (Get-ItemProperty $RunPath -ErrorAction SilentlyContinue).InputKey } else { $null }
        if ($null -eq $runDisabled) { break }
        Start-Sleep -Milliseconds 50
    }
    if ($null -ne $runDisabled) {
        throw "Start with Windows did not remove the HKCU Run value."
    }

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

    if (-not (Test-Path $RunPath)) { New-Item -Path $RunPath -Force | Out-Null }
    if ($null -ne $runBefore) {
        Set-ItemProperty -Path $RunPath -Name InputKey -Type String -Value $runBefore
    } else {
        Remove-ItemProperty -Path $RunPath -Name InputKey -ErrorAction SilentlyContinue
    }
    if ($null -ne $legacyRunBefore) {
        Set-ItemProperty -Path $RunPath -Name VietnameseKeyboard -Type String -Value $legacyRunBefore
    } else {
        Remove-ItemProperty -Path $RunPath -Name VietnameseKeyboard -ErrorAction SilentlyContinue
    }

    if ($registeredBefore) {
        if ($bindingBefore -and (Test-Path -LiteralPath $bindingBefore)) {
            & $Registrar --bind-only --dll $bindingBefore | Out-Null
        }
        if ($activeBefore) {
            & $Registrar --activate-only | Out-Null
        } else {
            & $Registrar --disable | Out-Null
        }
    } else {
        & $Registrar --disable | Out-Null
        & $Registrar --unregister | Out-Null
    }

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
