# InputKey for macOS

This leaf is a native InputMethodKit adapter over InputKey's Rust C ABI. It owns macOS event, preedit, and commit integration; Vietnamese typing rules remain in `Operators`.

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

## Literalize shortcut

The default whole-token shortcut is **Control+;** (physical key code 41 on ANSI Mac keyboards). It invokes `LiteralizeToken`: the current transformed token becomes its full raw physical keystrokes without inserting a semicolon. Example: `refer → rể → Control+; → refer`.

Open the InputKey menu and choose **Hoàn tác dấu của từ: <shortcut>** to record a new gesture. Press the desired modifier keys, then the non-modifier key; its keyDown is consumed. Escape cancels. At least one modifier is required. The menu checkbox enables or disables the shortcut, and **Reset shortcut → Control+;** restores the default.

You may choose Ctrl+Space in the recorder, but macOS may reserve it for switching input sources. If InputKey does not receive it, change or disable **Select the previous input source** in **System Settings → Keyboard → Keyboard Shortcuts → Input Sources**.

## Configuration

The InputKey menu provides Telex/VNI selection, Auto Restore, and shortcut preferences. Shortcut settings are persisted in `NSUserDefaults`; they do not rebuild or reset the Rust core.
