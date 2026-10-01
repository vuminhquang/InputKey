$ErrorActionPreference = 'Stop'
cargo build --release -p inputkey-wasm --target wasm32-unknown-unknown
New-Item -ItemType Directory -Force dist | Out-Null
Copy-Item target/wasm32-unknown-unknown/release/inputkey_wasm.wasm dist/inputkey.wasm -Force
