# InputKey

**Free & Open Source Vietnamese Keyboard**

> Gõ tiếng Việt theo tốc độ tư duy của bạn.

InputKey is a Rust-first Vietnamese input method with one typing engine shared across browser, Windows, Linux, and macOS.

## Platforms

- **Chrome / Edge** — Manifest V3 extension backed by Rust WebAssembly.
- **Windows** — native keyboard integration with a tray app.
- **Linux** — Fcitx5 and IBus adapters over the Rust C ABI.
- **macOS** — native InputMethodKit input method over the Rust C ABI.

## Whole-word undo

InputKey can restore the current token to the physical keys that produced it.

Example:

```text
physical keys: refer
rendered:      rể
shortcut:      whole-word undo
result:        refer
```

Defaults:

- Chrome / Edge: **Ctrl+Space**
- Windows: **Ctrl+Space**
- macOS: **Control+;**
- Linux: **Ctrl+Space**

The shortcut is configurable and can be disabled.

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
- whole-word undo

Regression coverage includes cases such as `pass → pas`, `passs → pass`, `password → password`, `urrl → url`, `assk → ask`, `affter → after`, `exxe → exe`, and `ajjax → ajax`.

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

Linux shortcut parser smoke test:

```sh
./Bootstrap/Build/test-linux-shortcut.sh
```

Linux Fcitx5 and IBus adapters are built with CMake after building `inputkey-cabi`.

GitHub Actions validates Linux, Windows, macOS, and Chromium builds. Version tags (`v*`) publish GitHub Release artifacts automatically.

## Source layout

- `Boundary/` — engine input/output contracts
- `CoreAbstractions/` — shared interfaces and state types
- `Operators/` — typing engine
- `Implementations/` — browser/OS integrations
- `Bootstrap/` — entry points and build tooling

See [ARCHITECTURE.md](ARCHITECTURE.md) for a short component overview.

## Development approach

InputKey is organized with **Abstract Driven Development (A.D.D) V3**:

- A.D.D V3: https://abstractdriven.com/
- A.D.D quick reference: https://abstractdriven.com/llms.txt

Related work from the same project:

- **XS3 — the first agent-centric language:** https://abstractdriven.com/xs3
- XS3 language reference: https://abstractdriven.com/llms-language.txt

These links provide attribution while this repository keeps its own architecture notes concise.

## License

InputKey is licensed under GPL-2.0-or-later. See [LICENSE](LICENSE) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Source: https://github.com/vuminhquang/InputKey
