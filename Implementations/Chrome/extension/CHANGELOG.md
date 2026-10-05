# 5.2.10

- Made root `VERSION` the release-version source of truth and added CI checks that keep Rust, Chromium, Linux CMake, and macOS bundle metadata synchronized with it; release tags must match `VERSION`.
- The default modifier-only `Ctrl+Shift` toggle now requires both modifiers from the same side of the keyboard: Left Ctrl + Left Shift or Right Ctrl + Right Shift.
- Windows TSF now finalizes an active composition during `OnTestKeyDown`, before the host processes the original physical Enter; this follows TSF's native keystroke test/edit-session path and does not synthesize or replay Enter.
- Documented TSF activation as best-effort: InputKey can maintain current-user registration/binding without elevation, while compatibility input remains available when Windows cannot activate the profile in a process/session.

# 5.2.8

- Added InputKey-defined Telex-style language packs for Danish, Swedish, and German, including case-preserving Germanic pair transforms.
- Extended runtime/catalog, native language DLL packaging, Chromium hints/tests, README, and the landing page for the new language packs.

# 5.2.7

- Added the InputKey landing page and GitHub Pages deployment.
- Preserved correction casing in the Vietnamese language machine.

# 5.2.6

- Restored `Start with Windows` to a direct current-user `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run` registration from the tray process; removed the separate startup helper and Startup-folder shortcut path that triggered Defender persistence heuristics.
- Kept TSF as the primary Windows path: InputKey automatically registers/repairs/enables/activates its Text Service profile for the current user without UAC, and `Remove Windows integration...` can unregister that user-wide TSF profile/COM binding.
- Compatibility fallback now admits generic user-session text targets to the worker capability probe, tries native range then UI Automation then eligible synthetic fallback, and no longer disables compatibility globally after one transport failure.

# 5.2.3

- Kept the 5.2.2 Vietnamese boundary fixes: physical consonant onsets are preserved, vowel-initial correction stays in the same vowel family, and `[` / `]` remain ordinary punctuation.
- Isolated `Start with Windows` into `InputKeyStartup.exe`. Normal `InputKey.exe` startup never creates or repairs autorun state; the helper creates or removes a per-user Startup-folder shortcut only after the user explicitly toggles `Start with Windows` in the tray UI.
- Moved the low-level compatibility hook and its fallback transports out of `InputKey.exe` into the lifecycle-bound `InputKeyCompatibility.exe` process. The tray/control executable no longer hosts global keyboard or mouse hooks in-process.
- Windows TSF and native language packs now live under `runtime/<version>/`. Portable upgrades stage the complete new runtime before switching the stable TSF binding, so new applications use the new version while already-open applications can finish with the runtime they already loaded.
- Added a guarded portable deployment flow that verifies package version and runtime bytes, preserves old runtimes, switches TSF only after staging succeeds, and refuses to overwrite a same-version runtime with different bytes.

# 5.2.2

- Boundary smart correction now preserves the physical initial consonant sequence, preventing cases such as `stop` being reinterpreted as `tóp` by consuming the leading `s` as a tone key.
- Vowel-initial correction is constrained to the same initial vowel family: `a` may become `a/ă/â`, `e` may become `e/ê`, `o` may become `o/ô/ơ`, and `u` may become `u/ư`; correction cannot invent a different initial consonant or vowel family.
- Removed the legacy `[` → `ư` and `]` → `ơ` Telex shortcuts; brackets now remain ordinary punctuation.

# 5.2.1

- Windows portable builds now keep all binaries in one self-contained folder, automatically rebind the existing TSF COM registration to the `InputKeyTSF.dll` beside the running `InputKey.exe`, and reuse the same stable TSF CLSID/profile across upgrades instead of creating version-specific registrations.
- `Turn off InputKey and exit` now persists the disabled state before the tray app closes; graceful `WM_CLOSE` follows the same rule so a vanished tray cannot leave InputKey logically enabled.
- Added an explicit `Remove Windows integration...` action that disables InputKey, removes Start with Windows, unregisters the TSF profile/COM class, and then exits without deleting the portable folder.
- Added configurable InputKey on/off shortcuts on Windows and Chromium. The default is `Ctrl+Shift`; examples such as `Alt+Z` and `Ctrl+Shift+K` are accepted, `Off` disables the shortcut, and `Ctrl+Alt` combinations are rejected to preserve AltGr.
- Renamed and explained Auto Restore as restoring the original physical keys when a Telex/VNI interpretation stops looking like valid Vietnamese, rather than leaving a mistaken conversion.

# 5.2.0

- Added native settings UI for Windows, macOS, and Linux while keeping the Chromium extension popup as the browser settings surface.
- Added a Chrome UI packaging contract so CI fails if the popup, manifest wiring, or required UI assets disappear.
- Windows settings now expose the active language, input method, Simple Telex, Auto Restore, Smart correction, and enable state through a native Win32 window opened from the tray menu.
- macOS now includes an AppKit settings window opened from the InputMethodKit menu, with live runtime reload after changes.
- Linux now packages a GTK settings application backed by the same persisted adapter settings; Fcitx5 and IBus reload persisted settings when activated or focused.

# 5.1.0

- Replaced the procedural Root-to-language API with semantic `RootTransition` events. Language packs now use transition-only ABI v3; the public C/WASM semantic ABI is v4. Space and punctuation remain distinct root states but share the language boundary policy, and command chords use `ShortcutBoundary`.

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
- Moved correction timing behind semantic word boundaries: `SpaceBoundary` and `PunctuationBoundary` are distinct root states but both emit the same boundary transition to the active language FSM. Vietnamese live typing stays sequential (`mor → mỏ`, `more → mỏe`), invalid speculative text rolls back on the next printable event, explicit repeat-cancel remains immediate (`exx → ex`), and only modifier intent may float at Space (`thuongwf → thường`) while base-letter order stays fixed (`mac`, `macos`, `mod`).
- Replaced native language-pack operation tables with transition-only ABI v3 so each language consumes the same semantic RootTransition stream while keeping its own internal FSM.
- Fixed Windows language switching after updates by rebinding the per-user TSF COM class to the packaged `InputKeyTSF.dll` before activation. Packaged rebuilds rotate any still-loaded previous runtime instead of leaving COM pointed at old `target` or legacy dist paths.
- Fixed Enter/Tab/navigation requiring a second key press after an active TSF composition. These inputs now enter `CaretMoveBoundary` with their original cause and continue to the application once after the composition commit.
- Unified mouse and keyboard caret relocation under `CaretMoveBoundary` while preserving each cause. Windows TSF tracks mouse activity across the active text context and commits displayed composition before the caret moves; Windows compatibility, Chromium, Linux, and macOS adapters use the same semantic contract.
- Fixed duplicate characters in TSF hosts when optional mouse tracking could not be attached. Mouse tracking now joins the same owned composition edit session and can no longer release an already handled physical key back to the application.
- Corrected TSF composition startup after a reboot exposed swallowed input in hosts such as Explorer and editors. Initial text is inserted first and the native composition is then started over that exact range; if a host rejects composition startup after insertion, InputKey keeps the physical key owned and resets logical composition instead of duplicating or losing the character. Caret and cleanup work after text mutation remain best-effort.
- Fixed duplicated physical characters such as `a` becoming `aa` when TSF had already changed application text but a later caret-placement or composition-cleanup operation failed. Composition now starts before application text is mutated, and every post-mutation step is non-owning/best-effort.
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
