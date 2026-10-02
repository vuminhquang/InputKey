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
