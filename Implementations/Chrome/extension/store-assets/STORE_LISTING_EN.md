# Chrome Web Store listing — English

## Title
InputKey — Vietnamese Keyboard

## Summary
Free and open-source Vietnamese typing for Telex and VNI. Processing stays on-device.

## Suggested category
Tools

## Detailed description
InputKey lets you type Vietnamese directly in editable fields on websites using Telex or VNI.

Key features:
- Telex and VNI Unicode input.
- Works with common input, textarea, and contenteditable fields.
- Toggle Vietnamese mode from the popup, Ctrl + Shift, or Alt + Z.
- Simple Telex, Backspace reconstruction, Esc raw-key restore, and deterministic Auto Restore.
- Whole-word undo: by default Ctrl+Space turns the current transformed token back into its physical keys, for example \`refer → rể → Ctrl+Space → refer\`.
- The whole-word shortcut is configurable, can be disabled, and is matched by physical key code.
- No ads, analytics, remote code, or developer-operated typing service.
- Typed content is processed in browser memory and is not stored or sent to the developer.
- Source code is public: https://github.com/vuminhquang/InputKey

Website access is used only to detect typing inside editable fields and convert Telex/VNI key sequences into Vietnamese. Password fields are ignored.

This is an in-browser typing helper, not a system-wide IME. Chrome-protected pages and some highly customized editors may not be supported.

## Developer / support contact
vu.minh.quang@outlook.com

### Repeat to cancel
Repeating a modifier to cancel is always available: \`docss → docs\`, \`tesst → test\`. English doubled letters in words such as “password” and “coffee” are preserved.

### Shared Rust engine
The Chrome/Edge extension uses the same deterministic Rust typing engine as the native InputKey builds, compiled to packaged local WebAssembly as \`inputkey.wasm\`. All typing remains on-device.
