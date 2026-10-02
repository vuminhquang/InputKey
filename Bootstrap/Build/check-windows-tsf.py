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
]:
    if required not in service:
        errors.append(f"WindowsTSF service missing lifecycle primitive: {required}")

if errors:
    print("Windows TSF contract violation:")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)

print("Windows TSF contract passed: composition-owned text, selection replacement, no injection/clipboard/timing workaround.")
