# InputKey Chrome

This leaf owns browser DOM integration only. Vietnamese typing policy lives in the Rust `inputkey-operators` FSM and is composed into the local `inputkey.wasm` artifact by `Bootstrap/WASM`.

The extension does not load remote code. The WASM module is packaged with the extension. The “Hoàn tác dấu của từ” shortcut is configurable in the popup and defaults to Ctrl+Space on Windows and in Chrome/Edge. It records the physical `KeyboardEvent.code`, requires at least one modifier, and maps to the semantic `LiteralizeToken` command (for example `rể` typed from `refer` becomes `refer`).

Build with `Bootstrap/Build/build-chrome.ps1`.
