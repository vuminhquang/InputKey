$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Version = (Get-Content (Join-Path $Root "VERSION") -Raw).Trim()
$Exe = Join-Path $Root "dist\InputKey-Windows-$Version\InputKey.exe"
if (-not (Test-Path $Exe)) {
    throw "Missing $Exe. Run build-windows.ps1 first."
}

$restartPaths = @()
Get-Process InputKey -ErrorAction SilentlyContinue | ForEach-Object {
    try {
        if ($_.Path) {
            $restartPaths += $_.Path
            Stop-Process -Id $_.Id -Force -ErrorAction Stop
            $_.WaitForExit()
        }
    } catch {
        throw "Could not stop existing InputKey instance: $($_.Exception.Message)"
    }
}
$restartPaths = @($restartPaths | Select-Object -Unique)

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
        throw "InputKey exited before the tray window became ready (exit code $($process.ExitCode))."
    }
    if ($window -eq [IntPtr]::Zero) {
        throw "InputKeyControlWindow did not appear."
    }

    $Registrar = Join-Path (Split-Path $Exe) "InputKeyTSFRegister.exe"
    $Dll = Join-Path (Split-Path $Exe) "InputKeyTSF.dll"
    if (Test-Path $Registrar) {
        $status = (& $Registrar --status 2>&1 | Out-String).Trim()
        if ($status -match "available=true") {
            $Inproc = "HKCU:\Software\Classes\CLSID\{5F4A4C92-85B3-4F69-A6BC-B427D51D5E50}\InprocServer32"
            $bound = (Get-ItemProperty $Inproc -ErrorAction Stop).'(default)'
            if ([IO.Path]::GetFullPath($bound) -ne [IO.Path]::GetFullPath($Dll)) {
                throw "Portable TSF rebind regression: registry points to '$bound' instead of '$Dll'."
            }
        }
    }

    if (-not [InputKeyWindowProbe]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
        throw "Could not send WM_CLOSE to InputKeyControlWindow."
    }

    if (-not $process.WaitForExit(8000)) {
        throw "InputKey did not terminate after WM_CLOSE; lifecycle regression detected."
    }
    if ($process.ExitCode -ne 0) {
        throw "InputKey exited with code $($process.ExitCode)."
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
        if (Test-Path $path) { Start-Process -FilePath $path | Out-Null }
    }
}
