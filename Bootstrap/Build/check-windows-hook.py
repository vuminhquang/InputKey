from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
source = root / "Implementations/WindowsHook/src/win32.rs"
text = source.read_text(encoding="utf-8")

match = re.search(r"(?s)(?:unsafe\s+)?extern\s+\"system\"\s+fn\s+keyboard_proc\b.*?\{", text)
if not match:
    raise SystemExit("Windows hook check failed: keyboard_proc callback is missing")

start = match.start()
depth = 0
body_end = None
for index in range(text.find("{", start), len(text)):
    char = text[index]
    if char == "{":
        depth += 1
    elif char == "}":
        depth -= 1
        if depth == 0:
            body_end = index + 1
            break

if body_end is None:
    raise SystemExit("Windows hook check failed: keyboard_proc body is incomplete")

body = text[start:body_end]
forbidden = (
    "SendInput",
    "EnginePort",
    "Mutex",
    "RwLock",
    ".recv(",
    "WaitFor",
    "sleep(",
    "File::",
    "OpenOptions",
    "RegSetValue",
    "RegCreate",
    "std::fs",
    "GetAsyncKeyState",
)
found = [word for word in forbidden if word in body]
if found:
    raise SystemExit(
        "Windows hook check failed: forbidden callback operations: " + ", ".join(found)
    )

if not re.search(r"HOOK_ID\s*=\s*id\b", text) or "GetCurrentThreadId()" not in text:
    raise SystemExit("Windows hook check failed: actual hook thread ID is not assigned")

injected = re.search(r"if data\.dwExtraInfo == EXTRA\s*\{([^}]*)\}", body, re.S)
if injected and re.search(r"\breturn\s+1\b", injected.group(1)):
    raise SystemExit("Windows hook check failed: self-injected input is suppressed")

if "thread::yield_now" in text:
    raise SystemExit("Windows hook check failed: busy ready wait")

production = text.lower()
for forbidden_product_feature in (
    "rail_window",
    "tscshellcontainerclass",
    "tscaxhostclass",
    "wslg",
    "remoteapp",
    "isremotewindow",
):
    if forbidden_product_feature in production:
        raise SystemExit(
            "Windows hook check failed: removed remote/browser handoff feature reappeared: "
            + forbidden_product_feature
        )

for forbidden_watchdog in ("settimer(", "wm_timer", "reinstall", "hookmonitor"):
    if forbidden_watchdog in production:
        raise SystemExit(
            "Windows hook check failed: timeout/watchdog recovery is forbidden: "
            + forbidden_watchdog
        )

print("Windows hook static source check passed")
