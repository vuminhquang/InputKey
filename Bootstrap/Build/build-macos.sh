#!/usr/bin/env sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
VERSION=$(tr -d '\r\n' < "$ROOT/VERSION")
cd "$ROOT"

cargo build --release -p inputkey-cabi

clang -fobjc-arc -framework Cocoa -framework InputMethodKit \
  -I Boundary/NativeABI \
  Implementations/macOS/InputMethodKit/main.m \
  Implementations/macOS/InputMethodKit/InputKeyInputController.m \
  -L target/release -linputkey_cabi \
  -Wl,-rpath,@executable_path/../Frameworks \
  -o /tmp/InputKey

APP="$ROOT/dist/macos/InputKey.app"
ZIP="$ROOT/dist/InputKey-macOS-$VERSION.zip"
rm -rf "$APP"
rm -f "$ZIP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/Frameworks"
cp /tmp/InputKey "$APP/Contents/MacOS/InputKey"
cp Implementations/macOS/InputMethodKit/Info.plist "$APP/Contents/Info.plist"
cp Implementations/Chrome/extension/icons/icon128.png "$APP/Contents/Resources/InputKey.png"
cp target/release/libinputkey_cabi.dylib "$APP/Contents/Frameworks/libinputkey_cabi.dylib"
install_name_tool -change @rpath/libinputkey_cabi.dylib @executable_path/../Frameworks/libinputkey_cabi.dylib "$APP/Contents/MacOS/InputKey"

test -x "$APP/Contents/MacOS/InputKey"
test -f "$APP/Contents/Info.plist"
test -f "$APP/Contents/Frameworks/libinputkey_cabi.dylib"

/usr/bin/ditto -c -k --sequesterRsrc --keepParent "$APP" "$ZIP"
echo "Built $APP"
echo "Built $ZIP"
