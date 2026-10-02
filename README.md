# InputKey

**Free & Open Source Multilingual Input Method**

> Gõ tiếng Việt theo tốc độ tư duy của bạn.

InputKey is a Rust-first multilingual input method with one language-independent root composition machine shared across browser, Windows, Linux, and macOS. Vietnamese is the default language; French Telex is bundled in 5.1.

## Platforms

- **Chrome / Edge** — Manifest V3 extension backed by Rust WebAssembly.
- **Windows** — TSF text service with automatic target-aware text transports and a tray app.
- **Linux** — Fcitx5 and IBus adapters over the Rust C ABI.
- **macOS** — native InputMethodKit input method over the Rust C ABI.

## Composition boundaries

InputKey follows normal IME boundary behavior instead of keeping a separate token mode. While a token is active:

- **Space** enters the root correction boundary: the active language gets one optional correction pass, the token is finalized, and a normal space is inserted. **Punctuation** finalizes without invoking correction, then inserts the delimiter.
- **Enter, Tab, arrows, Home/End, Page Up/Down, and Delete** commit exactly the text currently displayed, end composition, and let that same key continue to the application.
- **Shift+Space** is a one-shot raw escape: it commits the physical keystroke sequence for the current token, consumes the Space keystroke, and ends composition.

For example, `dd` displays `đ`; pressing an arrow commits `đ` and then moves the caret. `refer` may display `rể`; pressing Shift+Space commits `refer`.

## Windows input paths

Windows uses the TSF text service as the primary input path. Installing or updating the text service may require one UAC approval; normal typing and the tray app run as the current user. Applications that were already running when the text service was first installed may need to be restarted once so their TSF thread manager can load InputKey. This includes Windows Terminal if it was already open before InputKey's text service was installed.

When TSF is unavailable for the focused target, InputKey probes target capabilities instead of routing by application name. It prefers owned native edit ranges, then UI Automation editable patterns, and uses an isolated synthetic transport only for compatible fallback surfaces such as Remote Desktop or custom Chromium render surfaces. Clipboard-backed paste remains a guarded secondary transport for supported native edits. Password and read-only fields are excluded, injected InputKey events are ignored by the hook, and clipboard restoration never overwrites a newer clipboard change made by another application.

## Typing behavior

The shared engine supports:

- Telex and Simple Telex
- VNI
- tone placement and Vietnamese vowel shapes
- `dd → đ`
- `w` and bracket shortcuts
- repeat-to-cancel
- English collision recovery
- Backspace/Escape reconstruction
- natural IME composition boundaries and one-shot Shift+Space raw escape
- root-owned Space correction with optional language-specific correction rules, enabled by default for Vietnamese on Windows

At the Space boundary, the Vietnamese correction hook can recover modifier-order variants without inventing missing tone input, including `nhieue → nhiều`, `chueyern → chuyển`, and `dduocwj → được`. Live typing remains sequential; explicit repeat-cancel still happens immediately at the character event. Regression coverage also includes `pass → pas`, `passs → pass`, `password → password`, `urrl → url`, `assk → ask`, `affter → after`, `exxe → exe`, and `ajjax → ajax`.

## Privacy

InputKey performs typing conversion locally. The browser extension loads its packaged `inputkey.wasm` resource locally and does not need a remote typing service. Native adapters use the local Rust engine directly.

## Build and test

Rust stable is required for Rust targets.

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
python Bootstrap/Build/check-architecture.py
```

Chrome / Edge on Windows:

```powershell
.\Bootstrap\Build\build-chrome.ps1
```

Chrome / Edge on Unix-like hosts:

```sh
./Bootstrap/Build/build-chrome.sh
```

Windows:

```powershell
.\Bootstrap\Build\build-windows.ps1
.\Bootstrap\Build\test-windows-lifecycle.ps1
```

macOS:

```sh
./Bootstrap/Build/build-macos.sh
./Bootstrap/Build/install-macos.sh
```

Linux Fcitx5 and IBus adapters are built with CMake after building `inputkey-cabi`.

GitHub Actions validates Linux, Windows, macOS, and Chromium builds. Version tags (`v*`) publish GitHub Release artifacts automatically.

## Source layout

- `Boundary/` — engine input/output contracts
- `CoreAbstractions/` — shared interfaces and state types
- `Operators/` — language-independent root composition machine
- `Implementations/Languages/` — language-specific child machines
- `Implementations/LanguagePackLoader/` — native dynamic language-pack discovery
- `Implementations/` — browser/OS integrations
- `Bootstrap/` — entry points and build tooling

See [ARCHITECTURE.md](ARCHITECTURE.md) for a short component overview.

## License

InputKey is licensed under GPL-2.0-or-later. See [LICENSE](LICENSE) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Source: https://github.com/vuminhquang/InputKey
