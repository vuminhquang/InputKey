from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
hook_source = root / "Implementations/WindowsHook/src/win32.rs"
resolver_source = root / "Implementations/WindowsHook/src/transport.rs"
native_source = root / "Implementations/WindowsHook/src/native.rs"
uia_source = root / "Implementations/WindowsHook/src/uia.rs"
synthetic_source = root / "Implementations/WindowsHook/src/synthetic.rs"
clipboard_source = root / "Implementations/WindowsClipboard/src/lib.rs"
transition_source = root / "Implementations/WindowsHook/src/transition.rs"

hook = hook_source.read_text(encoding="utf-8")
resolver = resolver_source.read_text(encoding="utf-8")
native = native_source.read_text(encoding="utf-8")
uia = uia_source.read_text(encoding="utf-8")
synthetic = synthetic_source.read_text(encoding="utf-8")
clipboard = clipboard_source.read_text(encoding="utf-8")
transition = transition_source.read_text(encoding="utf-8")

match = re.search(
    r'(?s)(?:unsafe\s+)?extern\s+"system"\s+fn\s+keyboard_proc\b.*?\{',
    hook,
)
if not match:
    raise SystemExit("Windows compatibility check failed: keyboard_proc is missing")

start = match.start()
depth = 0
body_end = None
for index in range(hook.find("{", start), len(hook)):
    char = hook[index]
    if char == "{":
        depth += 1
    elif char == "}":
        depth -= 1
        if depth == 0:
            body_end = index + 1
            break

if body_end is None:
    raise SystemExit("Windows compatibility check failed: keyboard_proc body is incomplete")

body = hook[start:body_end]
forbidden_callback = (
    "ClipboardPaste",
    "TypingEnginePort",
    "Mutex",
    "RwLock",
    "WaitFor",
    "Ole",
    "OpenClipboard",
    "SetClipboardData",
    "SendInput",
    "keybd_event",
    "RegSetValue",
    "RegCreate",
    "std::fs",
    "sleep(",
)
found = [word for word in forbidden_callback if word.lower() in body.lower()]
if found:
    raise SystemExit(
        "Windows compatibility check failed: blocking/mutating callback operations: "
        + ", ".join(found)
    )

for forbidden in (
    "SendInput",
    "keybd_event",
    "KEYEVENTF_UNICODE",
    "SetTimer(",
    "WM_TIMER",
    "watchdog",
    "hookmonitor",
    "reinstall",
):
    if forbidden.lower() in hook.lower():
        raise SystemExit(
            "Windows compatibility check failed: leaf transport leaked into hook orchestration: "
            + forbidden
        )

for required in (
    "LLKHF_INJECTED",
    "may_support_text",
    "capture(target",
    "FinalizeWithDelimiter",
):
    if required not in hook:
        raise SystemExit(
            "Windows compatibility check failed: hook orchestration primitive missing: "
            + required
        )

for required in (
    "EM_GETSEL",
    "EM_SETSEL",
    "EM_REPLACESEL",
    "ES_PASSWORD",
    "ES_READONLY",
):
    if required not in native:
        raise SystemExit(
            "Windows compatibility check failed: native-range safety primitive missing: "
            + required
        )

for required in (
    "CurrentIsPassword",
    "CurrentIsReadOnly",
    "CurrentHasKeyboardFocus",
    "CurrentProcessId",
    "GetFocusedElement",
    "ValuePattern",
    "TextPattern",
):
    if required not in uia:
        raise SystemExit(
            "Windows compatibility check failed: UI Automation capability guard missing: "
            + required
        )

for required in (
    "thread_has_tsf",
    "native::OwnedRange::capture",
    "capture_focused",
    "OwnedTransport::Automation",
    "OwnedTransport::Synthetic",
    "chrome_widgetwin_1",
    "rail_window",
    "tscshellcontainerclass",
    "tscaxhostclass",
):
    if required.lower() not in resolver.lower():
        raise SystemExit(
            "Windows compatibility check failed: capability resolver primitive missing: "
            + required
        )

native_pos = resolver.find("native::OwnedRange::capture")
uia_pos = resolver.find("capture_focused")
synthetic_pos = resolver.rfind("synthetic::OwnedSynthetic::capture")
if not (0 <= native_pos < uia_pos < synthetic_pos):
    raise SystemExit(
        "Windows compatibility check failed: resolver order must prefer native range, "
        "then UI Automation, then synthetic fallback"
    )

for required in (
    "SendInput",
    "KEYEVENTF_UNICODE",
    "GetForegroundWindow",
    "GetAncestor",
    "INPUTKEY_SYNTHETIC_TAG",
):
    if required not in synthetic:
        raise SystemExit(
            "Windows compatibility check failed: isolated synthetic transport missing: "
            + required
        )

for forbidden in (
    "SetTimer(",
    "WM_TIMER",
    "watchdog",
    "OpenClipboard",
    "SetClipboardData",
):
    if forbidden.lower() in synthetic.lower():
        raise SystemExit(
            "Windows compatibility check failed: synthetic leaf contains unrelated mechanism: "
            + forbidden
        )

for required in (
    "OleGetClipboard",
    "OleSetClipboard",
    "GetClipboardSequenceNumber",
    "WM_PASTE",
):
    if required not in clipboard:
        raise SystemExit(
            "Windows compatibility check failed: clipboard preservation missing: "
            + required
        )

if "decision_boundary" not in transition:
    raise SystemExit(
        "Windows compatibility check failed: delimiter semantics bypass the root engine"
    )

print(
    "Windows compatibility transport check passed: callback stays nonblocking; "
    "resolver prefers native range/UIA and isolates synthetic fallback."
)
