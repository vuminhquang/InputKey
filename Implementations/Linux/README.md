# Linux adapters

Fcitx5 and IBus are platform implementations over the stable C ABI in `Boundary/NativeABI`. They own preedit/commit and Linux UI integration only. The root composition machine lives in `Operators`; language-specific rules live in `Implementations/Languages`.

- `Fcitx5/`: preferred on modern Linux/Wayland setups.
- `IBus/`: GNOME/Ubuntu-friendly adapter.
- `Settings/`: shared persisted language/method/options.

## Composition boundaries

Space and punctuation finalize the current word before the delimiter continues. Arrow/navigation keys, Tab, and Enter commit the currently displayed preedit and then pass the original key through to the application. Shift+Space inserts ordinary Space when displayed equals raw; otherwise it commits the physical keystrokes without adding Space and ends the preedit. Language, method, and language-owned options are derived from the runtime language catalog.
