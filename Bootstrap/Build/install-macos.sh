#!/usr/bin/env sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
APP="$ROOT/dist/macos/InputKey.app"
DEST="$HOME/Library/Input Methods/InputKey.app"
test -d "$APP" || { echo "Build InputKey.app first" >&2; exit 1; }
mkdir -p "$(dirname "$DEST")"
STAGE="$DEST.installing.$$"
rm -rf "$STAGE"
cp -R "$APP" "$STAGE"
rm -rf "$DEST"
mv "$STAGE" "$DEST"
echo "Installed $DEST"
