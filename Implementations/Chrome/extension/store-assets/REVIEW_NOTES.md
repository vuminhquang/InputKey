# Notes for Chrome Web Store review — InputKey 5.0.0

**Primary purpose:** local Vietnamese input method for editable fields on websites using Telex or VNI.

## How to test

1. Install the extension.
2. Open a normal HTTP/HTTPS page containing an input or textarea.
3. Type `tieengs Vieetj` and observe `tiếng Việt`.
4. Type `dduwowngf` and observe `đường`.
5. Type physical keys `refer`; InputKey renders `rể`.
6. Press the configured whole-word shortcut (default Ctrl+Space); the token becomes `refer`.
7. Open the popup to change/reset/disable that shortcut or switch Telex/VNI.

## Data handling

Keystrokes and the current token are processed locally in browser memory. Typed text is not stored, transmitted, sold, or shared. Password fields are ignored. Only extension preferences are saved with `chrome.storage.local`.

## Network behavior

The extension makes no external network requests. It loads the packaged local Rust WebAssembly file with `fetch(chrome.runtime.getURL("inputkey.wasm"))`. There is no XHR/WebSocket, analytics, ads, telemetry, remotely hosted code, or developer-operated typing service.

## Broad site access rationale

The extension’s single purpose requires its content script to run on ordinary HTTP/HTTPS pages where users type. It does not request `file://` access or access to protected browser pages.

## Implementation

Version 5.0.0 uses a packaged Rust WebAssembly typing engine. JavaScript handles browser integration, settings, and UI.

Source code: https://github.com/vuminhquang/InputKey

## Developer contact

vu.minh.quang@outlook.com
