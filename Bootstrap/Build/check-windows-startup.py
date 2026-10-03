from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
CONTROL_IMPL = ROOT / "Implementations" / "WindowsControl"
BOOTSTRAP = ROOT / "Bootstrap" / "WindowsControl"
MAIN = BOOTSTRAP / "src" / "main.rs"
STARTUP = CONTROL_IMPL / "src" / "startup.rs"
CONTROL = CONTROL_IMPL / "src" / "win32.rs"
BOOT_CARGO = BOOTSTRAP / "Cargo.toml"
BUILD = ROOT / "Bootstrap" / "Build" / "build-windows.ps1"
DEPLOY = ROOT / "Bootstrap" / "Build" / "deploy-windows-portable.ps1"

errors: list[str] = []

user_scope_roots = [
    BOOTSTRAP / "src",
    ROOT / "Bootstrap" / "WindowsTSF" / "src",
    CONTROL_IMPL / "src",
    ROOT / "Implementations" / "WindowsTSF" / "src",
]
for scope in user_scope_roots:
    for source in scope.rglob("*.rs"):
        text = source.read_text(encoding="utf-8-sig")
        for forbidden in [
            'wide("runas")',
            "HKEY_LOCAL_MACHINE",
            "HKLM:",
            "requireAdministrator",
            "highestAvailable",
        ]:
            if forbidden in text:
                errors.append(
                    f"{source.relative_to(ROOT)} violates user-only Windows scope: {forbidden}"
                )

if not STARTUP.exists():
    errors.append("Windows startup implementation is missing")
    startup = ""
else:
    startup = STARTUP.read_text(encoding="utf-8")

for required in [
    r"Software\Microsoft\Windows\CurrentVersion\Run",
    "HKEY_CURRENT_USER",
    "RegSetValueExW",
    "RegCreateKeyExW",
    "RegDeleteValueW",
    'const VALUE_NAME: &str = "InputKey"',
    'const LEGACY_VALUE_NAME: &str = "VietnameseKeyboard"',
    "remove_stale_shortcut",
]:
    if required not in startup:
        errors.append(f"direct HKCU Run startup contract missing: {required}")

for forbidden in ["IShellLinkW", "ShellLink", "persist.Save(", "HKEY_LOCAL_MACHINE"]:
    if forbidden in startup:
        errors.append(f"startup persistence must remain user-only: {forbidden}")

control = CONTROL.read_text(encoding="utf-8")
for required in [
    "ID_STARTUP",
    'wide("Start with Windows")',
    "crate::startup::is_enabled()",
    "crate::startup::set_enabled(!enabled)",
]:
    if required not in control:
        errors.append(f"Windows UI missing explicit startup action: {required}")

main = MAIN.read_text(encoding="utf-8-sig")
for forbidden in [
    "InputKeyStartup.exe",
    "startup_helper_path",
    "set_startup_enabled(",
    "startup_enabled()",
    'wide("runas")',
    "ShellExecuteExW",
]:
    if forbidden in main:
        errors.append(f"InputKey bootstrap violates user-only startup contract: {forbidden}")

boot_cargo = BOOT_CARGO.read_text(encoding="utf-8")
if "InputKeyStartup" in boot_cargo:
    errors.append("startup helper binary is still declared in Bootstrap/WindowsControl/Cargo.toml")

deploy = DEPLOY.read_text(encoding="utf-8-sig")
for forbidden in [
    r"CurrentVersion\Run",
    "Set-ItemProperty",
    "New-ItemProperty",
    "Remove-ItemProperty",
    "Start-Process -Verb RunAs",
]:
    if forbidden in deploy:
        errors.append(f"portable deploy must not own startup persistence/elevation: {forbidden}")

for required in [
    "ObsoleteStartupHelper",
    "Remove-Item -LiteralPath $ObsoleteStartupHelper",
]:
    if required not in deploy:
        errors.append(f"portable deploy must remove obsolete startup helper: {required}")

build = BUILD.read_text(encoding="utf-8")
if "InputKeyStartup" in build:
    errors.append("Windows package still builds or includes InputKeyStartup.exe")

if errors:
    print("Windows startup persistence contract violation:")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)

print(
    "Windows startup persistence contract passed: explicit UI changes HKCU Run directly; "
    "TSF/startup stay in current-user scope, normal startup/deploy remain persistence-neutral, "
    "and no startup helper is packaged."
)
