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

- one language-independent root FSM for semantic input and lifecycle
- semantic root states: Idle, Composing, SpaceBoundary, PunctuationBoundary, CaretMoveBoundary, ShortcutBoundary, CompositionControl, RawBoundary, and Lifecycle
- the root enters a state before synchronously emitting `RootTransition` to the active language machine
- one Root-to-language contract: `accepts_character`, `on_transition`, and `state`
- language-specific FSMs remain inside each language pack; the root does not know Vietnamese/French transformation rules
- Space and punctuation are separate root states but may share the same language boundary policy
- mouse and navigation inputs share `CaretMoveBoundary` while preserving the original cause
- command chords use `ShortcutBoundary`; AltGraph remains character input
- Backspace/Escape are composition-control events
- Shift+Space is a one-shot raw boundary
- native dynamic language packs use transition-only ABI v3; the public C/WASM semantic ABI is v4
- Telex, Simple Telex, VNI, tone/vowel-shape handling, repeat-to-cancel, English collision recovery, and replay remain language/runtime features


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
