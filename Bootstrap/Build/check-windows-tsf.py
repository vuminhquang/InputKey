from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
BASES = [ROOT / "Implementations" / "WindowsTSF", ROOT / "Bootstrap" / "WindowsTSF"]
FORBIDDEN = {
    "synthetic keyboard injection": r"\bSendInput\b|\bkeybd_event\b|\bKEYEVENTF_",
    "clipboard mutation": r"\bSetClipboardData\b|\bEmptyClipboard\b|\bOpenClipboard\b",
    "timing workaround": r"\bSleep\s*\(|\bsleep\s*\(|\btimeout\b|\bwatchdog\b",
}

errors: list[str] = []
for base in BASES:
    for source in base.rglob("*"):
        if not source.is_file() or source.suffix.lower() not in {".rs", ".toml"}:
            continue
        text = source.read_text(encoding="utf-8")
        for label, pattern in FORBIDDEN.items():
            if re.search(pattern, text, re.IGNORECASE):
                errors.append(f"{source.relative_to(ROOT)}: forbidden {label}")

service = (ROOT / "Implementations" / "WindowsTSF" / "src" / "service.rs").read_text(encoding="utf-8")
for required in [
    "StartComposition",
    "SetText",
    "EndComposition",
    "OnCompositionTerminated",
    "ValidateCaret",
    "InsertTextAtSelection",
    "TF_IAS_NO_DEFAULT_COMPOSITION",
    "ITfKeyTraceEventSink",
    "OnKeyTraceDown",
    "ITfMouseSink",
    "ITfMouseTracker",
    "AdviseMouseSink",
    "OnMouseEvent",
    "RootInput::CaretMoveBoundary",
    "CaretMoveCause::Mouse",
    "RootInput::ShortcutBoundary",
    "RootInput::SpaceBoundary",
    "RootInput::PunctuationBoundary",
]:
    if required not in service:
        errors.append(f"WindowsTSF service missing lifecycle primitive: {required}")

if "let _ = self.install_mouse_tracking(ec, mouse_sink);" not in service:
    errors.append("WindowsTSF mouse tracking must be best-effort inside the owned update edit session")
for required in ["self.context.GetStart(ec)?", "self.context.GetEnd(ec)?", "range.ShiftEndToRange(ec, &end, TF_ANCHOR_END)?"]:
    if required not in service:
        errors.append(f"WindowsTSF mouse tracking must cover the whole document when the host supports it: {required}")
if "ensure_mouse_tracking" in service:
    errors.append("WindowsTSF must not open a second post-key mouse-tracking edit session")

if "InsertTextAtSelection(ec, TF_IAS_NO_DEFAULT_COMPOSITION, &utf16)" not in service:
    errors.append("WindowsTSF must insert initial text before starting its composition")
if "return Ok(None);" not in service:
    errors.append("WindowsTSF must retain key ownership when composition startup fails after insertion")
if "let _ = self.set_caret_at_end(ec, &range);" not in service:
    errors.append("WindowsTSF caret placement after SetText must not release key ownership")
if "let _ = composition.EndComposition(ec);" not in service:
    errors.append("WindowsTSF cleanup after SetText must not release key ownership")

registration = (ROOT / "Implementations" / "WindowsTSF" / "src" / "registration.rs").read_text(encoding="utf-8")
registrar = (ROOT / "Bootstrap" / "WindowsTSF" / "src" / "bin" / "register.rs").read_text(encoding="utf-8")
control = (ROOT / "Bootstrap" / "WindowsControl" / "src" / "main.rs").read_text(encoding="utf-8")
control_ui = (ROOT / "Implementations" / "WindowsControl" / "src" / "win32.rs").read_text(encoding="utf-8")
build = (ROOT / "Bootstrap" / "Build" / "build-windows.ps1").read_text(encoding="utf-8")
deploy = (ROOT / "Bootstrap" / "Build" / "deploy-windows-portable.ps1").read_text(encoding="utf-8-sig")
bootstrap_tsf = (ROOT / "Bootstrap" / "WindowsTSF" / "src" / "lib.rs").read_text(encoding="utf-8")

for label, text in [
    ("registration", registration),
    ("registrar", registrar),
    ("packaged control", control),
]:
    for forbidden in [
        'wide("runas")',
        "ShellExecuteExW",
        "HKEY_LOCAL_MACHINE",
        "HKLM:",
        ".Register(&CLSID_INPUTKEY_TEXT_SERVICE)",
        ".AddLanguageProfile(",
        "RegisterProfile(",
    ]:
        if forbidden in text:
            errors.append(f"{label} violates current-user TSF contract: {forbidden}")

for label, text, required in [
    ("registration", registration, r"Software\Microsoft\CTF\TIP"),
    ("registration", registration, "HKEY_CURRENT_USER"),
    ("registration", registration, "text_service_registered"),
    ("registration", registration, "register_user_tip_state"),
    ("registration", registration, "unregister_user_tip_state"),
    ("registration", registration, "REG_DWORD"),
    ("registration", registration, "register_text_service"),
    ("registration", registration, ".RegisterCategory("),
    ("registration", registration, ".EnableLanguageProfile("),
    ("registration", registration, "unregister_text_service"),
    ("registration", registration, ".RemoveLanguageProfile("),
    ("registration", registration, ".Unregister(&CLSID_INPUTKEY_TEXT_SERVICE)"),
    ("registration", registration, ".UnregisterCategory("),
    ("registrar", registrar, "--register-only"),
    ("registrar", registrar, "--unregister"),
    ("registrar", registrar, "--bind-only"),
    ("registrar", registrar, "--activate-only"),
    ("registrar", registrar, "--disable"),
    ("packaged control", control, "ensure_packaged_text_service"),
    ("packaged control", control, "run_registrar"),
    ("packaged control", control, "--register-only"),
    ("packaged control", control, "--unregister"),
    ("packaged control", control, "install_text_service"),
    ("packaged control", control, "runtime_directory"),
    ("Windows UI", control_ui, "Install Text Service..."),
    ("Windows UI", control_ui, "Remove Windows integration..."),
    ("Windows build", build, "InputKeyCompatibility.exe"),
    ("Windows build", build, "runtime\\"),
    ("portable deploy", deploy, "legacy_runtimes=preserved"),
    ("portable deploy", deploy, "--bind-only"),
    ("TSF bootstrap", bootstrap_tsf, "tsf_factory_selects_french_language"),
]:
    if required not in text:
        errors.append(f"{label} missing current-user TSF invariant: {required}")

if "let _ = ensure_packaged_text_service();" not in control:
    errors.append("InputKey startup must automatically register/repair the current-user TSF service and request activation")

if "inputkey_windows_tsf::register_text_service(" in control:
    errors.append("InputKey.exe must isolate TSF registration in InputKeyTSFRegister.exe")

if "installed_now = !inputkey_windows_tsf::text_service_registered()" not in control:
    errors.append("Install/Repair Text Service must distinguish a new current-user registration")

for required in [
    "test_phase_commit_then_pass_key",
    "commit_test_phase_boundary",
    "Microsoft TSF permits edit-session work during the test phase",
    "return Ok(BOOL::from(false));",
]:
    if required not in service:
        errors.append(
            f"WindowsTSF Enter must commit during OnTestKeyDown then pass the original physical key: {required}"
        )

if errors:
    print("Windows TSF contract violation:")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)

print(
    "Windows TSF contract passed: TSF remains the primary current-user path; "
    "InputKey auto-registers/repairs its user-wide profile without elevation and requests activation, "
    "Enter commits in the TSF test phase before the original physical key reaches the host, "
    "whole-document mouse tracking is best-effort, "
    "and compatibility remains available when Windows or the host does not provide those TSF capabilities."
)
