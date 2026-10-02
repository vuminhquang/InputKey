# 5.1.0

- Added a language-independent root composition machine with language child machines.
- Added French Telex: `ee → ê`, `es → é`, `ef → è`, `oe → œ`, `ae → æ`, `cc → ç`, and related French accents.
- Added native dynamic language packs and metadata-driven language/method/options UI.
- Made Space/Shift+Space/navigation boundary semantics shared across languages.
- Reworked Windows compatibility input into capability-based native range, UI Automation, and isolated synthetic transports.
- Added compatibility support for custom Chromium render surfaces such as Zalo without app-specific routing.
- Fixed Windows Start/Search compatibility by sharing one STA worker apartment between OLE clipboard fallback and UI Automation.
- Prevented compatibility transports from replaying a physical key after a target write already occurred, eliminating duplicated characters in Windows Search.
- Fixed native dynamic language packs so each key/backspace/escape/finalize event is applied exactly once; this removes truncated UTF-8 replacement glyphs such as broken `dd → đ` in Windows Search.
- Added capability-based fallback for keyboard-focused UI Automation text surfaces such as Windows Terminal: when TSF and writable ranges are unavailable, InputKey can keep composition through its isolated synthetic transport.
- Moved correction timing into the language-independent root FSM: only the Space decision boundary offers the active language one correction pass. Vietnamese live typing stays sequential (`mor → mỏ`, `more → mỏe`), invalid speculative text rolls back on the next printable event, explicit repeat-cancel remains immediate (`exx → ex`), and only modifier intent may float at Space (`thuongwf → thường`) while base-letter order stays fixed (`mac`, `macos`, `mod`).
- Split native language-pack finalization from boundary correction in language-pack ABI v2 so every language can independently define or omit its own correction policy.
- Fixed Windows language switching after updates by rebinding the per-user TSF COM class to the packaged `InputKeyTSF.dll` before activation. Packaged rebuilds rotate any still-loaded previous runtime instead of leaving COM pointed at old `target` or legacy dist paths.
- Fixed Enter/Tab/navigation requiring a second key press after an active TSF composition. Natural keyboard boundaries now enter the root `NaturalBoundary` state from the non-filtering TSF key-trace observer, so the same physical key continues to the application once.
- Added a distinct root `MouseBoundary` state for pointer/caret relocation. Windows TSF tracks mouse activity across the active text context and commits displayed composition before the caret moves; the Windows compatibility hook, Chrome/Edge, and macOS adapters route their real mouse events through the same root boundary contract. The old direct `commit_displayed` API was removed.
- Updated Chrome/Edge WASM to use the same language catalog and root composition contract.

# 5.0.0

- Rewrote the production engine and native integrations in Rust.
- Preserved Telex, Simple Telex, VNI, repeat-cancel, Auto Restore, and existing regression behavior.
- Added whole-word undo: `refer → rể → shortcut → refer`.
- Added configurable whole-word shortcuts. Browser/Windows default to Ctrl+Space; macOS defaults to Control+;.
- Chrome/Edge now use the packaged Rust `inputkey.wasm` engine.
- Added native Windows, macOS InputMethodKit, Linux Fcitx5, and Linux IBus builds.
- Added architecture checks and Linux/Windows/macOS CI.

# 4.1.4

- Fixed repeat-cancel continuation, including the `urrl → url` family of cases.
- Kept English recovery working for words such as `password`.
- Added regression coverage for repeated tone modifiers.

# 4.1.3

- Fixed repeat-cancel for `[` and `]` shortcuts.

# 4.1.1

- Made repeat-to-cancel always available, for example `docss → docs`.
- Removed the old Smart double-cancel option.
- Improved late-shape cancellation, navigation behavior, and visible version information.
