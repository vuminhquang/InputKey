from pathlib import Path

root = Path(__file__).resolve().parents[2]

adapters = {
    "Fcitx5": (root / "Implementations/Linux/Fcitx5/inputkey.cpp").read_text(encoding="utf-8"),
    "IBus": (root / "Implementations/Linux/IBus/inputkey.c").read_text(encoding="utf-8"),
}

required_semantic_calls = (
    "inputkey_caret_move_boundary_utf8",
    "inputkey_raw_boundary",
    "inputkey_shortcut_boundary",
    "inputkey_composition_control_utf8",
    "inputkey_accepts_character_utf8",
    "inputkey_character_utf8",
    "inputkey_space_boundary",
    "inputkey_punctuation_boundary_utf8",
    "inputkey_lifecycle_utf8",
)

for name, source in adapters.items():
    missing = [call for call in required_semantic_calls if call not in source]
    if missing:
        raise SystemExit(
            f"Linux adapter contract failed: {name} is missing semantic routes: "
            + ", ".join(missing)
        )

    raw = source.find("inputkey_raw_boundary")
    shortcut = source.find("inputkey_shortcut_boundary")
    character = source.find("inputkey_accepts_character_utf8")
    if not (0 <= raw < shortcut < character):
        raise SystemExit(
            f"Linux adapter contract failed: {name} must classify raw and command "
            "boundaries before normal character input"
        )

    if "ctrl && alt" not in source:
        raise SystemExit(
            f"Linux adapter contract failed: {name} does not preserve Ctrl+Alt/AltGr "
            "as character input"
        )

for cause in (
    "left",
    "right",
    "up",
    "down",
    "home",
    "end",
    "page_up",
    "page_down",
    "insert",
    "delete",
    "tab",
    "enter",
):
    for name, source in adapters.items():
        if f'"{cause}"' not in source:
            raise SystemExit(
                f"Linux adapter contract failed: {name} is missing caret cause {cause}"
            )

settings_ui_path = root / "Implementations/Linux/Settings/inputkey_settings_app.c"
if not settings_ui_path.is_file():
    raise SystemExit("Linux adapter contract failed: native settings app is missing")
settings_ui = settings_ui_path.read_text(encoding="utf-8")
for token in (
    "gtk_window_new",
    "inputkey_linux_settings_load",
    "inputkey_linux_settings_save",
    "inputkey_language_count",
    "inputkey_method_count",
    "inputkey_option_count",
):
    if token not in settings_ui:
        raise SystemExit(
            f"Linux adapter contract failed: native settings app is missing {token}"
        )

if "inputkey_linux_settings_load(&settings_)" not in adapters["Fcitx5"]:
    raise SystemExit("Linux adapter contract failed: Fcitx5 does not reload persisted settings")
if "reload_persisted_settings" not in adapters["IBus"]:
    raise SystemExit("Linux adapter contract failed: IBus does not reload persisted settings")

legacy = (
    "ShortcutConfig",
    "inputkey_shortcut_reload",
    "inputkey_shortcut_match_configured",
)
for name, source in adapters.items():
    found = [token for token in legacy if token in source]
    if found:
        raise SystemExit(
            f"Linux adapter contract failed: {name} still references removed shortcut "
            "configuration: " + ", ".join(found)
        )

print(
    "Linux adapter contract passed: Fcitx5 and IBus route physical input through "
    "the semantic C ABI, keep AltGr out of command shortcuts, and package a native "
    "settings surface backed by persisted adapter settings."
)
