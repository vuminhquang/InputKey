#!/usr/bin/env sh
set -eu
cargo build --release -p inputkey-wasm --target wasm32-unknown-unknown
mkdir -p dist
cp target/wasm32-unknown-unknown/release/inputkey_wasm.wasm dist/inputkey.wasm
