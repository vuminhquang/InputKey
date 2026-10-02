# InputKey Architecture

InputKey uses one language-independent root composition machine across browser and native platforms. The selected child machine owns language-specific interpretation.

## Source map

- `Boundary/` — input commands and data structures.
- `CoreAbstractions/` — shared interfaces and state types.
- `Operators/` — root composition machine, session, and replay.
- `Implementations/Languages/Vietnamese/` — Vietnamese child machine.
- `Implementations/Languages/French/` — French Telex child machine.
- `Implementations/LanguagePackLoader/` — native dynamic language-pack discovery.
- `Implementations/Chrome/` — browser extension.
- `Implementations/WindowsTSF/` — Windows TSF text service.
- `Implementations/WindowsHook/` — compatibility input capture and automatic target transport selection when TSF is unavailable.
- `Implementations/WindowsClipboard/` — clipboard-backed compatibility text transport.
- `Implementations/WindowsSettings/` — Windows user settings.
- `Implementations/WindowsControl/` — tray and control-plane integration.
- `Implementations/Linux/Fcitx5/` — Fcitx5 integration.
- `Implementations/Linux/IBus/` — IBus integration.
- `Implementations/macOS/InputMethodKit/` — macOS input method.
- `Implementations/EnglishBloom/` — English word filter.
- `Implementations/EventBus/` — event publisher implementation.
- `Bootstrap/WASM/` — WebAssembly target.
- `Bootstrap/CABI/` — native C ABI target.
- `Bootstrap/WindowsControl/` — Windows tray/control executable.
- `Bootstrap/WindowsTSF/` — TSF DLL and registration helper.
- `Bootstrap/Build/` — build and verification scripts.

## Engine features

- Telex
- Simple Telex
- VNI
- tone and vowel-shape handling
- repeat-to-cancel
- English collision recovery
- Backspace, Escape, finalize, reset, and replay
- one logical input event per FSM transition
- root-owned Space correction phase with optional language-specific correction rules
- natural composition boundaries with a one-shot raw-key boundary; punctuation/finalize do not invoke Space correction

## Platform targets

- Chrome / Edge: Rust WebAssembly
- Windows: TSF text service with capability-based native/UI Automation/clipboard/synthetic compatibility transports
- Linux: Rust C ABI + Fcitx5 / IBus adapters
- macOS: Rust C ABI + InputMethodKit

## Build outputs

- Chromium extension ZIP
- Windows executable, TSF DLL, registration helper, and native language-pack DLLs
- Linux native library/adapters
- macOS InputMethodKit app bundle

CI builds and tests Linux, Windows, macOS, and Chromium targets.
