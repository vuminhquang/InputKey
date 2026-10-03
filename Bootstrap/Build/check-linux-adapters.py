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
    "the semantic C ABI and keep AltGr out of command shortcuts."
)
