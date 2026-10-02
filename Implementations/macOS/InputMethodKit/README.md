# InputKey for macOS

This leaf is a native InputMethodKit adapter over InputKey's Rust C ABI. It owns macOS event, preedit, commit, and menu integration. The root composition machine and language-specific child machines remain in Rust.

## Build

Run on macOS:

```sh
./Bootstrap/Build/build-macos.sh
```

The script builds the local Rust C ABI library, compiles the Objective-C sources with ARC, and creates an unsigned local app bundle at `dist/macos/InputKey.app`.

## Install for local testing

```sh
mkdir -p "$HOME/Library/Input Methods"
rm -rf "$HOME/Library/Input Methods/InputKey.app"
cp -R dist/macos/InputKey.app "$HOME/Library/Input Methods/"
```

Then log out/in (or restart the text input services), open **System Settings → Keyboard → Text Input → Edit**, and add **InputKey**.

## Composition boundaries

Space and punctuation finalize the current word. Arrow/navigation keys, Tab, Return, and keypad Enter commit the currently displayed marked text and then let the original key continue to the client. Shift+Space is a fixed one-shot raw boundary: it commits only the physical keystroke sequence, consumes the Space keystroke, and ends composition.

## Configuration

The InputKey menu is generated from the language catalog and provides language, method, and language-owned options. These preferences are persisted in `NSUserDefaults`; the Objective-C adapter does not contain Vietnamese or French typing rules.
