# Languages and host integrations

InputKey currently bundles Vietnamese, French, Danish, Swedish and German. Language interpretation remains inside each language pack, behind the same root transition contract.

- Chromium: MV3 content script, packaged local WASM and extension-local configuration.
- Windows: TSF primary; compatibility capture/transport fallback selected by target capabilities, with protected fields excluded. Portable runtime versions are staged before user-level TSF binding changes.
- Linux: C ABI with Fcitx5 and IBus adapters.
- macOS: C ABI with InputMethodKit.

See [README](../../../README.md), [ARCHITECTURE](../../../ARCHITECTURE.md), and corresponding module READMEs for the detailed behavior and current artifact paths. Product release and live installation are distinct verification claims.
