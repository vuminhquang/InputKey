# InputKey WindowsHook

This crate is the Windows native transport leaf. It owns Win32 integration, a bounded
SPSC queue, exact-modifier shortcut matching, and the minimal `EnginePort` contract.
The Bootstrap crate adapts `Session` to that contract; Vietnamese typing policy remains
in Operators. Literalize defaults to Ctrl+Space; its recorder accepts a physical virtual
key only when at least one modifier is held and Escape cancels. User preferences belong
under the per-user `HKCU\\Software\\InputKey\\Settings` key. Startup registration uses
the per-user Run key and an explicitly quoted executable path.

Browser, RDP, WSLg, RemoteApp, and remote-window handoff behavior is not part of this app.
Shutdown must be state/signaled and joined; hook callbacks must never invoke engine or
synthetic-input functions.
