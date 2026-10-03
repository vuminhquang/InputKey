$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Version = (Get-Content (Join-Path $Root "VERSION") -Raw).Trim()
$Exe = Join-Path $Root "dist\InputKey-Windows-$Version\InputKey.exe"
if (-not (Test-Path $Exe)) {
    throw "Missing $Exe. Run build-windows.ps1 first."
}

$restoreRunningInstance = $false
Get-Process InputKey -ErrorAction SilentlyContinue | ForEach-Object {
    try {
        if ($_.Path -and ([IO.Path]::GetFullPath($_.Path) -eq [IO.Path]::GetFullPath($Exe))) {
            $restoreRunningInstance = $true
            Stop-Process -Id $_.Id -Force -ErrorAction Stop
            $_.WaitForExit()
        }
    } catch {
        throw "Could not stop existing canonical InputKey instance: $($_.Exception.Message)"
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

    if (-not [InputKeyWindowProbe]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
        throw "Could not send WM_CLOSE to InputKeyControlWindow."
    }

    if (-not $process.WaitForExit(8000)) {
        throw "InputKey did not terminate after WM_CLOSE; lifecycle regression detected."
    }
    if ($process.ExitCode -ne 0) {
        throw "InputKey exited with code $($process.ExitCode)."
    }
    Write-Host "windows-lifecycle=ok"
} finally {
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
    if ($restoreRunningInstance) {
        Start-Process -FilePath $Exe | Out-Null
    }
}
