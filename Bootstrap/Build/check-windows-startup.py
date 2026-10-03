from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
CONTROL_IMPL = ROOT / "Implementations" / "WindowsControl"
BOOTSTRAP = ROOT / "Bootstrap" / "WindowsControl"
MAIN = BOOTSTRAP / "src" / "main.rs"
HELPER = BOOTSTRAP / "src" / "bin" / "startup.rs"
CONTROL = CONTROL_IMPL / "src" / "win32.rs"
BUILD = ROOT / "Bootstrap" / "Build" / "build-windows.ps1"
DEPLOY = ROOT / "Bootstrap" / "Build" / "deploy-windows-portable.ps1"

errors: list[str] = []

for source in list(CONTROL_IMPL.rglob("*.rs")) + [MAIN]:
    text = source.read_text(encoding="utf-8")
    for forbidden in [
        "CurrentVersion\\Run",
        "RegSetValueExW",
        "RegCreateKeyExW",
        "RegDeleteValueW",
        "IShellLinkW",
        "InputKey.lnk",
    ]:
        if forbidden in text:
            errors.append(
                f"{source.relative_to(ROOT)} embeds startup persistence primitive {forbidden}"
            )

if not HELPER.exists():
    errors.append("startup helper source is missing")
    helper = ""
else:
    helper = HELPER.read_text(encoding="utf-8")

for required in [
    "IShellLinkW",
    "InputKey.lnk",
    'join("Startup")',
    'arg == "--enable"',
    'arg == "--disable"',
    'arg == "--status"',
]:
    if required not in helper:
        errors.append(f"startup helper missing explicit persistence contract: {required}")

for forbidden in ["CurrentVersion\\Run", "RegSetValueExW", "RegCreateKeyExW", "RegDeleteValueW"]:
    if forbidden in helper:
        errors.append(
            f"startup helper must use the Startup shortcut instead of legacy Run-key primitive {forbidden}"
        )

control = CONTROL.read_text(encoding="utf-8")
for required in ["ID_STARTUP", 'wide("Start with Windows")', "startup_enabled", "set_startup_enabled"]:
    if required not in control:
        errors.append(f"Windows UI missing explicit startup action: {required}")
for forbidden in ["Configure Start with Windows...", "shell:startup"]:
    if forbidden in control:
        errors.append(f"Windows UI must not fall back to manual startup configuration: {forbidden}")

main = MAIN.read_text(encoding="utf-8")
if "set_startup_enabled(true)" in main or "set_startup_enabled(false)" in main:
    errors.append("InputKey bootstrap must not mutate autorun implicitly")
for required in ["InputKeyStartup.exe", 'command.arg("--enable")', 'command.arg("--disable")']:
    if required not in main:
        errors.append(f"InputKey bootstrap must delegate explicit startup changes to helper: {required}")

deploy = DEPLOY.read_text(encoding="utf-8")
for forbidden in ["CurrentVersion\\Run", "Set-ItemProperty", "New-ItemProperty", "Remove-ItemProperty"]:
    if forbidden in deploy:
        errors.append(
            f"portable deploy must not change Start with Windows implicitly: {forbidden}"
        )
if "InputKeyStartup.exe" not in deploy:
    errors.append("portable deploy must carry InputKeyStartup.exe without invoking it")

build = BUILD.read_text(encoding="utf-8")
if "InputKeyStartup.exe" not in build or "--bin InputKeyStartup" not in build:
    errors.append("Windows package must build and include InputKeyStartup.exe")

if errors:
    print("Windows startup persistence contract violation:")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)

print(
    "Windows startup persistence contract passed: normal startup and deploy do not change autorun; explicit UI changes are isolated in InputKeyStartup.exe using a Startup shortcut."
)
