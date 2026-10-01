# Linux adapters

Fcitx5 and IBus are platform implementations over the stable C ABI in `Boundary/NativeABI`. They own preedit/commit integration only; all Vietnamese rules stay in Operators.

- `Fcitx5/`: preferred on modern Linux/Wayland setups.
- `IBus/`: GNOME/Ubuntu-friendly adapter.

## Literalize shortcut

Both adapters default to **Ctrl+Space** for `LiteralizeToken`. To change it, create
`$XDG_CONFIG_HOME/inputkey/config` (or `~/.config/inputkey/config` when
`XDG_CONFIG_HOME` is unset) with one setting, for example:

```ini
literalize_shortcut=Control+Shift+Semicolon
```

Modifiers are `Ctrl` (also spelled `Control`), `Alt`, `Shift`, and `Super`. Keys
are `Space`, `Semicolon`, or one ASCII printable character, such as `x`:

```ini
literalize_shortcut=Alt+x
```

Set `literalize_shortcut=disabled` to turn the shortcut off. Delete the setting
line or the config file to restore Ctrl+Space. The adapters read the setting at
startup and on reset; changes take effect on the next engine reset/startup.
Shift+letter remains ordinary typing unless
that exact chord is configured.

Run the standalone Linux parser/matcher check with
`Bootstrap/Build/test-linux-shortcut.sh` (requires a C compiler).
