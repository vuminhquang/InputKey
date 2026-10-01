#!/bin/sh
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT HUP INT TERM
cc -std=c99 -Wall -Wextra -Werror \
  "$repo_root/Implementations/Linux/ShortcutConfig/shortcut_config.c" \
  "$repo_root/Implementations/Linux/ShortcutConfig/test_shortcut_config.c" \
  -o "$tmp_dir/test-linux-shortcut"
"$tmp_dir/test-linux-shortcut"
