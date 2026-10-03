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

def callback_body(name: str) -> str:
    match = re.search(
        rf'(?s)(?:unsafe\s+)?extern\s+"system"\s+fn\s+{re.escape(name)}\b.*?\{{',
        hook,
    )
    if not match:
        raise SystemExit(f"Windows compatibility check failed: {name} is missing")
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
        raise SystemExit(f"Windows compatibility check failed: {name} body is incomplete")
    return hook[start:body_end]

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
for callback in ("keyboard_proc", "mouse_proc"):
    body = callback_body(callback)
    found = [word for word in forbidden_callback if word.lower() in body.lower()]
    if found:
        raise SystemExit(
            f"Windows compatibility check failed: {callback} has blocking/mutating operations: "
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
    "WH_MOUSE_LL",
    "CaretMoveBoundary",
    "ShortcutBoundary",
    "CompositionControl",
    "RawBoundary",
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
    "CurrentIsKeyboardFocusable",
    "CurrentIsEnabled",
    "CurrentProcessId",
    "GetFocusedElement",
    "ValuePattern",
    "TextPattern",
    "UIA_TextControlTypeId",
    "UIA_DocumentControlTypeId",
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
    "focused_keyboard_text_surface",
    "windows.ui.input.inputsite.windowclass",
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

if "_automation_ready" in resolver or "|| automation_ready" not in resolver:
    raise SystemExit(
        "Windows compatibility check failed: generic HWNDs cannot reach the UI Automation capability probe"
    )

for required in (
    "CAPABILITY_UNKNOWN",
    "CAPABILITY_SUPPORTED",
    "CAPABILITY_UNSUPPORTED",
    "set_capability",
    "invalidate_capability",
    "replay_physical",
):
    if required not in hook:
        raise SystemExit(
            "Windows compatibility check failed: compatibility fallback state is missing: "
            + required
        )

composition_control = re.search(
    r"(?s)EventKind::CompositionControl\s*=>\s*\{(.*?)EventKind::RawBoundary",
    hook,
)
if not composition_control:
    raise SystemExit(
        "Windows compatibility check failed: CompositionControl branch is missing"
    )
if "capture_enabled.store(false" in composition_control.group(1):
    raise SystemExit(
        "Windows compatibility check failed: one transport failure disables compatibility globally"
    )

for required in (
    "SendInput",
    "KEYEVENTF_UNICODE",
    "KEYEVENTF_SCANCODE",
    "replay_physical",
    "GetForegroundWindow",
    "GetAncestor",
    "INPUTKEY_SYNTHETIC_TAG",
    "owner: HWND",
    "root: HWND",
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

for required in ("RootInput", "engine.dispatch(input)"):
    if required not in transition:
        raise SystemExit(
            "Windows compatibility check failed: semantic input bypasses root dispatch: "
            + required
        )

owned_write = re.search(
    r"(?s)let ok = owned\.as_mut\(\).*?range\.replace\(.*?if !ok \{(.*?)\}\s*else if remains_active",
    hook,
)
if not owned_write:
    raise SystemExit(
        "Windows compatibility check failed: owned transport write branch is missing"
    )
if "replay_literal" in owned_write.group(1):
    raise SystemExit(
        "Windows compatibility check failed: a key is replayed after an owned transport "
        "already attempted to mutate the target"
    )

for required in (
    "CurrentValue",
    "actual != next",
    "Clone(caret anchor)",
):
    if required not in uia:
        raise SystemExit(
            "Windows compatibility check failed: UI Automation write/caret invariant missing: "
            + required
        )

main_control = (root / "Bootstrap/WindowsControl/src/main.rs").read_text(encoding="utf-8")
compatibility_bootstrap = (root / "Bootstrap/WindowsControl/src/bin/compatibility.rs").read_text(encoding="utf-8")
windows_build = (root / "Bootstrap/Build/build-windows.ps1").read_text(encoding="utf-8")

if "inputkey_windows_hook::start" in main_control:
    raise SystemExit(
        "Windows compatibility check failed: InputKey.exe must not host the global compatibility hook in-process"
    )
for required in (
    "inputkey_windows_hook::start",
    "--parent",
    "WaitForMultipleObjects",
    "InputKey.Compatibility.Reload",
):
    if required not in compatibility_bootstrap:
        raise SystemExit(
            "Windows compatibility check failed: isolated compatibility process missing: "
            + required
        )
if "InputKeyCompatibility.exe" not in windows_build:
    raise SystemExit(
        "Windows compatibility check failed: Windows package must include InputKeyCompatibility.exe"
    )

print(
    "Windows compatibility transport check passed: callback stays nonblocking; "
    "resolver prefers native range/UIA, synthetic fallback stays isolated, and the global hook runs outside InputKey.exe."
)
