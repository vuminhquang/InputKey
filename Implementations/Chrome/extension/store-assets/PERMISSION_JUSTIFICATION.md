# Permission and site-access justification

## Single purpose
Provide Vietnamese Telex/VNI typing in editable fields on websites.

## `storage`
Used only to save local extension preferences: enabled state, input method (Telex/VNI), Simple Telex, auto-restore, and toast preference. The extension uses `chrome.storage.local`, not a developer server.

## Site access: `http://*/*` and `https://*/*`
The extension must run a content script on normal websites so it can receive keyboard events in editable fields and replace the current word with the corresponding Vietnamese Unicode text. This access is necessary for the extension's core user-facing feature.

The content script ignores password fields. It does not collect page contents, browsing history, or typed text; it does not send data off-device; and it does not inject ads or tracking.

## Remote code
None. All executable JavaScript required by the extension is packaged inside the submitted ZIP.

Developer/support contact: vu.minh.quang@outlook.com
