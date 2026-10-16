# Notes for Chrome Web Store review — InputKey 5.2.13

**Single purpose:** Local multilingual typing inside editable website fields. The popup exposes Vietnamese (Telex/VNI), French, Danish, Swedish and German (Telex).

## How to test
1. Install this update into the existing InputKey extension.
2. Open an ordinary HTTP/HTTPS page with a normal input or textarea (not a password field).
3. Select Vietnamese in the popup and type `tieengs Vieetj` to get `tiếng Việt`; type physical `queue` and see `queue` preserved on Space.
4. Type `as` to get `á`; Shift+Space restores the raw keys `as` without adding a space.
5. Change language in the popup: French `ee` → `ê`; Danish `ae` → `æ`; Swedish `ae` → `ä`; German `aw` → `ä`.
6. Configure a toggle shortcut or other language-specific options if desired.

## Data handling
Keystrokes and the current composition are handled only in browser memory. Typed text is not stored, transmitted, sold or shared. Password fields are ignored. Only extension preferences are stored using `chrome.storage.local`.

## Network behavior
No external calls. `fetch(chrome.runtime.getURL("inputkey.wasm"))` loads only the packaged local Rust WebAssembly. No remote code, backend typing service, analytics, ads or telemetry.

## Broad site access
The single multilingual typing purpose requires a content script on normal HTTP/HTTPS websites with user-editable fields. It does not request `file://` access or access to protected browser pages.

Website: https://vuminhquang.github.io/InputKey/
Privacy: https://vuminhquang.github.io/InputKey/privacy.html
Source: https://github.com/vuminhquang/InputKey
Developer contact: vu.minh.quang@outlook.com
