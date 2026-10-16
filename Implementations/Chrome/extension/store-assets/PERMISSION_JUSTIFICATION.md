# Permission and site-access justification

## Single purpose
Provide local multilingual text input in editable website fields, with supported Vietnamese Telex/VNI and French, Danish, Swedish and German Telex mappings.

## `storage`
Used only for local extension preferences: enabled state, language/method, language-specific options, auto restore, smart correction, toggle shortcut and toast preference. The extension uses `chrome.storage.local`, not a developer server.

## Site access: `http://*/*` and `https://*/*`
The content script must receive keyboard input in the user's editable fields on ordinary websites and replace only the active composition with the selected language's locally generated Unicode text. This is its core user-facing function, regardless of website domain.

Password fields are ignored. It does not collect or transmit page contents, browsing history or typed text. It does not inject advertising or tracking.

## Remote code
None. The Rust engine is compiled into the packaged local `inputkey.wasm` and all executable JavaScript ships inside the extension ZIP.

Website: https://vuminhquang.github.io/InputKey/
Privacy: https://vuminhquang.github.io/InputKey/privacy.html
Developer contact: vu.minh.quang@outlook.com
