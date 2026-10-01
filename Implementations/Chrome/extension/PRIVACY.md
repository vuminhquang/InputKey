# InputKey Privacy

InputKey processes keystrokes locally in the browser to provide Vietnamese Telex/VNI composition. Typed text is not sent to a server by the extension. InputKey has no analytics, ads, telemetry, remote-code loading, or account system.

The packaged `inputkey.wasm` file is loaded from the extension bundle through `chrome.runtime.getURL`; this is a local extension-resource fetch, not an external network request.

The `storage` permission stores user preferences such as enabled state and input method. It is not used to store typed content.
