# InputKey

**Free & Open Source Multilingual Input Method**

> Gõ tiếng Việt theo tốc độ tư duy của bạn.

InputKey is a Rust-first multilingual input method with one language-independent root composition machine shared across browser, Windows, Linux, and macOS. Vietnamese is the default language; InputKey also bundles French, Danish, Swedish, and German Telex-style language packs.

## Platforms

- **Chrome / Edge** — Manifest V3 extension backed by Rust WebAssembly.
- **Windows** — TSF text service with automatic target-aware text transports and a tray app.
- **Linux** — Fcitx5 and IBus adapters over the Rust C ABI.
- **macOS** — native InputMethodKit input method over the Rust C ABI.

## Composition boundaries

InputKey uses one semantic root state machine across every platform. Physical input is classified into `Character`, `SpaceBoundary`, `PunctuationBoundary`, `CaretMoveBoundary`, `ShortcutBoundary`, `CompositionControl`, `RawBoundary`, or `Lifecycle`. The root enters that state first, then synchronously emits a `RootTransition` to the active language machine.

- **Character input** stays in composition and is interpreted by the selected language FSM.
- **Space** enters `SpaceBoundary`; **punctuation** such as `. , ! ? ; :` enters the distinct `PunctuationBoundary`. They remain separate semantic states, but both ask the language FSM to resolve its word-boundary policy before the root emits the original delimiter. Vietnamese therefore applies the same correction policy to `dduwocj ` and `dduwocj.`.
- **Mouse clicks, Enter, Tab, arrows, Home/End, Page Up/Down, Delete, and Insert** enter one `CaretMoveBoundary` state while retaining the original cause. The language commits the currently displayed composition, then the original caret-moving event continues to the application.
- **Ctrl/Alt/Win command chords** enter `ShortcutBoundary`: the displayed composition is committed first, then the original shortcut continues unchanged. AltGraph is treated as character input, not as a shortcut.
- **Backspace and Escape** are `CompositionControl` events.
- **Shift+Space** enters `RawBoundary`: it commits the physical keystroke sequence for the current token and consumes the Space keystroke.
- Focus loss, context destruction, disable, language changes, finalize, and reset are explicit **Lifecycle** events.

For example, if `dd` displays `đ`, Left Arrow commits `đ` before the caret moves; clicking elsewhere does the same through `CaretMoveBoundary(Mouse)`. `Ctrl+A` commits the displayed token before Select All runs. `refer` may display a transformed candidate, while Shift+Space commits the physical `refer`.

The root owns semantic routing and lifecycle only. Vietnamese, French, Danish, Swedish, German, and future language packs keep their own language-specific FSMs and receive `RootTransition` events through one transition entrypoint instead of operation-specific methods.


## Windows input paths

Windows uses the TSF text service as the primary input path. InputKey automatically registers, repairs, enables, and activates its Text Service profile for the current user without elevation; the profile and COM binding live in the current-user Windows registration scope. Applications that were already open before registration may need to be restarted once so their TSF thread manager can load InputKey.

The packaged Windows runtime keeps TSF binaries and native language packs under `runtime/<version>/`. Portable deployment stages and verifies the complete new runtime first, then switches the current-user COM binding to that version. Starting `InputKey.exe` idempotently repairs the current-user TSF registration/binding and activates the profile; it never requires UAC or an administrator token. Already-open applications may keep their previously loaded version until they close, and old runtime folders are never overwritten during that handoff.

Closing the tray app gracefully turns InputKey off before exit but keeps the current-user TSF registration intact. `Remove Windows integration...` is the explicit cleanup path: it turns InputKey off, removes Start with Windows, unregisters the current user's InputKey Text Service profile/COM binding, and exits without deleting the portable folder. Normal `InputKey.exe` startup never creates or repairs autorun state. The tray's `Start with Windows` checkbox is the explicit user action that creates or removes InputKey's current-user `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run` value directly; no startup helper process or Startup-folder shortcut is used. The default on/off shortcut is `Ctrl+Shift`; it can be changed in Settings to combinations such as `Alt+Z` or `Ctrl+Shift+K`, or set to `Off`. `Ctrl+Alt` combinations are rejected so AltGr remains character input.

When TSF is unavailable for the focused target, the lifecycle-bound `InputKeyCompatibility.exe` helper probes target capabilities instead of routing by application name. The tray/control `InputKey.exe` does not host the global keyboard or mouse hooks itself. Compatibility prefers owned native edit ranges, then UI Automation editable patterns, and uses an isolated synthetic transport only for compatible fallback surfaces such as Remote Desktop or custom Chromium render surfaces. Clipboard-backed paste remains a guarded secondary transport for supported native edits. Password and read-only fields are excluded, injected InputKey events are ignored by the hook, and clipboard restoration never overwrites a newer clipboard change made by another application.

## Typing behavior

The shared engine supports:

- Telex and Simple Telex
- VNI
- tone placement and Vietnamese vowel shapes
- `dd → đ`
- `w`-based Vietnamese shape input
- repeat-to-cancel
- English collision recovery
- Backspace/Escape reconstruction
- semantic caret/shortcut/composition/lifecycle boundaries and one-shot Shift+Space raw escape
- language-owned boundary correction triggered by both Space and punctuation boundaries

At Space or punctuation boundaries, the Vietnamese FSM can recover modifier-order variants without inventing missing tone input, including `nhieue → nhiều`, `chueyern → chuyển`, `dduocwj → được`, and `dduwocj → được`. Boundary correction preserves the physical initial prefix: consonant onsets stay intact (`stop → stop`), while vowel-initial candidates remain in the same family (`a/ă/â`, `e/ê`, `o/ô/ơ`, `u/ư`, `i`, `y`). `[` and `]` are ordinary punctuation, not Vietnamese shape shortcuts. Live typing remains sequential; explicit repeat-cancel still happens immediately at the character event.

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
