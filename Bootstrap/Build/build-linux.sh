#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
VERSION=$(tr -d '\r\n' < "$ROOT/VERSION")
DIST="$ROOT/dist/linux"
STAGE="$ROOT/dist/.linux-stage"
FCITX_BUILD="$ROOT/target/fcitx5"
IBUS_BUILD="$ROOT/target/ibus"

cd "$ROOT"

cargo build --release -p inputkey-cabi
python3 Bootstrap/Build/cabi-smoke.py

rm -rf "$STAGE" "$FCITX_BUILD" "$IBUS_BUILD"
mkdir -p "$STAGE" "$DIST"

cmake -S Implementations/Linux/Fcitx5 -B "$FCITX_BUILD" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX=/usr \
  -DINPUTKEY_NATIVE_LIB_DIR="$ROOT/target/release"
cmake --build "$FCITX_BUILD" --parallel
DESTDIR="$STAGE" cmake --install "$FCITX_BUILD"

cmake -S Implementations/Linux/IBus -B "$IBUS_BUILD" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX=/usr \
  -DINPUTKEY_NATIVE_LIB_DIR="$ROOT/target/release"
cmake --build "$IBUS_BUILD" --parallel
DESTDIR="$STAGE" cmake --install "$IBUS_BUILD"

mkdir -p "$STAGE/usr/lib" "$STAGE/usr/share/doc/inputkey"
cp "$ROOT/target/release/libinputkey_cabi.so" "$STAGE/usr/lib/"
cp "$ROOT/README.md" "$ROOT/LICENSE" "$STAGE/usr/share/doc/inputkey/"

ZIP="$DIST/InputKey-Linux-x86_64-$VERSION.zip"
rm -f "$ZIP"
(
  cd "$STAGE"
  zip -qr "$ZIP" .
)
rm -rf "$STAGE"

echo "Built $ZIP"
