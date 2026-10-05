#!/usr/bin/env sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
VERSION=$(tr -d '\r\n' < "$ROOT/VERSION")
EXT="$ROOT/Implementations/Chrome/extension"
DIST="$ROOT/dist/chrome"
STAGE="$ROOT/dist/.chrome-stage"

python3 "$ROOT/Bootstrap/Build/sync-version.py" --check
python3 "$ROOT/Bootstrap/Build/check-chrome-ui.py"

rustup target add wasm32-unknown-unknown
cargo build --release -p inputkey-wasm --target wasm32-unknown-unknown --manifest-path "$ROOT/Cargo.toml"
cp "$ROOT/target/wasm32-unknown-unknown/release/inputkey_wasm.wasm" "$EXT/inputkey.wasm"
INPUTKEY_WASM="$EXT/inputkey.wasm" node "$EXT/content_test.cjs"

rm -rf "$STAGE"
mkdir -p "$STAGE" "$DIST"
cp -R "$EXT"/. "$STAGE"/
rm -f "$STAGE/content_test.cjs"
(
  cd "$STAGE"
  zip -qr "$DIST/InputKey-$VERSION.zip" .
)
cp "$DIST/InputKey-$VERSION.zip" "$DIST/InputKey.zip"
rm -rf "$STAGE"
echo "Built $DIST/InputKey-$VERSION.zip"
