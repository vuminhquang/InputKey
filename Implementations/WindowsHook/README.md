# InputKey WindowsHook

This crate captures compatibility input only when the focused Windows target cannot use the InputKey TSF path. Target handling is automatic; users do not choose a compatibility transport.

The hook callback only classifies and queues physical input. Text mutation stays on the worker. Supported native Edit/RichEdit/Windows Forms controls use owned selection ranges first and guarded clipboard paste only as a secondary transport. Windows Search is handled through UI Automation. Explicit Remote Desktop window classes (`RAIL_WINDOW`, `TscShellContainerClass`, and `TscAxHostClass`) may use the isolated synthetic transport as a last resort; InputKey-tagged injected input is ignored by the low-level hook.

Password and read-only native edit controls are excluded. Queue back-pressure fails open instead of blocking the hook thread. Shutdown is signaled and joined; there is no timer/watchdog transport recovery loop.

## Desktop/session lifecycle

Windows may silently remove a low-level keyboard hook if its callback cannot complete before the OS deadline. A surviving `InputKeyCompatibility.exe` process therefore does not prove its hook remains installed. Console HWNDs can also survive a lock/unlock while an earlier negative UI Automation capability remains cached. On a tray-observed Windows session unlock/reconnect or system resume, the control layer sends the existing compatibility reload event. The helper clears its Root composition, transport ownership, target capability and physical modifier/suppressed key-up tracking, then sends `REARM_HOOK_MESSAGE` to the original hook thread. That thread installs a fresh keyboard+mouse hook pair before releasing the old pair; a failed replacement leaves the old pair intact. The tray's separate toggle-shortcut hook is also reinstalled. This is driven by real OS lifecycle events, never elapsed time, polling or process self-restart.

The Rust Root FSM is unchanged; the repair belongs to Windows platform ownership, not Vietnamese language rules. Browser and TSF transports remain independent.
