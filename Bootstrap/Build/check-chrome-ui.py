from __future__ import annotations

import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
EXT = ROOT / "Implementations" / "Chrome" / "extension"

required_files = {
    "manifest.json",
    "popup.html",
    "popup.css",
    "popup.js",
    "background.js",
    "content.js",
    "engine_wasm.js",
}

errors: list[str] = []
for name in sorted(required_files):
    if not (EXT / name).is_file():
        errors.append(f"missing Chrome extension UI/runtime asset: {name}")

manifest_path = EXT / "manifest.json"
if manifest_path.is_file():
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("manifest_version") != 3:
        errors.append("Chrome extension must remain Manifest V3")
    action = manifest.get("action")
    if not isinstance(action, dict):
        errors.append("Chrome extension action block is missing")
    elif action.get("default_popup") != "popup.html":
        errors.append("Chrome extension action.default_popup must remain popup.html")

popup_path = EXT / "popup.html"
if popup_path.is_file():
    popup = popup_path.read_text(encoding="utf-8")
    for token in (
        'id="toggle"',
        'id="language"',
        'id="method"',
        'id="languageOptions"',
        'id="showToast"',
        'id="try"',
        'src="popup.js"',
        'href="popup.css"',
    ):
        if token not in popup:
            errors.append(f"Chrome popup contract missing: {token}")

popup_js = EXT / "popup.js"
if popup_js.is_file():
    source = popup_js.read_text(encoding="utf-8")
    for token in (
        "chrome.runtime.getManifest().version",
        "chrome.storage.local",
        "InputKey.catalog()",
        "new InputKey.Engine",
    ):
        if token not in source:
            errors.append(f"Chrome popup behavior contract missing: {token}")

if errors:
    print("Chrome extension UI contract failed.")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)

print("Chrome extension UI contract passed: popup and settings surface are packaged.")
