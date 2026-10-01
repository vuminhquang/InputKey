# InputKey Architecture

InputKey uses one Rust typing engine across browser and native platforms.

## References

- **Abstract Driven Development (A.D.D) V3:** https://abstractdriven.com/
- **A.D.D V3 quick reference:** https://abstractdriven.com/llms.txt
- **XS3 — the first agent-centric language:** https://abstractdriven.com/xs3
- **XS3 language reference:** https://abstractdriven.com/llms-language.txt

## Source map

- `Boundary/` — input commands and data structures.
- `CoreAbstractions/` — shared interfaces and state types.
- `Operators/` — Vietnamese typing engine, session, and replay.
- `Implementations/Chrome/` — browser extension.
- `Implementations/WindowsHook/` — Windows integration.
- `Implementations/Linux/Fcitx5/` — Fcitx5 integration.
- `Implementations/Linux/IBus/` — IBus integration.
- `Implementations/macOS/InputMethodKit/` — macOS input method.
- `Implementations/EnglishBloom/` — English word filter.
- `Implementations/EventBus/` — event publisher implementation.
- `Bootstrap/WASM/` — WebAssembly target.
- `Bootstrap/CABI/` — native C ABI target.
- `Bootstrap/WindowsHook/` — Windows executable.
- `Bootstrap/Build/` — build and verification scripts.

## Engine features

- Telex
- Simple Telex
- VNI
- tone and vowel-shape handling
- repeat-to-cancel
- English collision recovery
- Backspace, Escape, finalize, reset, and replay
- whole-word undo

## Platform targets

- Chrome / Edge: Rust WebAssembly
- Windows: native Rust executable
- Linux: Rust C ABI + Fcitx5 / IBus adapters
- macOS: Rust C ABI + InputMethodKit

## Build outputs

- Chromium extension ZIP
- Windows executable
- Linux native library/adapters
- macOS InputMethodKit app bundle

CI builds and tests Linux, Windows, macOS, and Chromium targets.
