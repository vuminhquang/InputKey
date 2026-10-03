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
build = (ROOT / "Bootstrap" / "Build" / "build-windows.ps1").read_text(encoding="utf-8")
bootstrap_tsf = (ROOT / "Bootstrap" / "WindowsTSF" / "src" / "lib.rs").read_text(encoding="utf-8")

for label, text, required in [
    ("registration", registration, "bind_text_service_dll"),
    ("registrar", registrar, "--bind-only"),
    ("packaged control", control, "bind_packaged_text_service"),
    ("packaged control", control, 'directory.join("languages")'),
    ("Windows build", build, "-loaded-"),
    ("TSF bootstrap", bootstrap_tsf, "tsf_factory_selects_french_language"),
]:
    if required not in text:
        errors.append(f"{label} missing canonical multilingual TSF invariant: {required}")

if errors:
    print("Windows TSF contract violation:")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)

print("Windows TSF contract passed: composition-owned text, selection replacement, no injection/clipboard/timing workaround.")
