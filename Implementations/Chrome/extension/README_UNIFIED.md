# InputKey Chrome

This leaf owns browser DOM integration only. Language-independent composition semantics live in the Rust root machine in `Operators`, while language-specific interpretation lives in `Implementations/Languages`. The root and bundled language children are composed into the local `inputkey.wasm` artifact by `Bootstrap/WASM`.

The extension does not load remote code. The WASM module is packaged with the extension. Space and punctuation finalize the current word; navigation and Enter keep the currently displayed composition while allowing the original key to continue. Shift+Space commits only the current token's physical keystream as a one-shot raw boundary; it does not insert a space.

Build with `Bootstrap/Build/build-chrome.ps1`.
