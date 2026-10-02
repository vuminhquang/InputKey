# InputKey WindowsHook

This crate captures compatibility input only when the focused Windows target cannot use the InputKey TSF path. Target handling is automatic; users do not choose a compatibility transport.

The hook callback only classifies and queues physical input. Text mutation stays on the worker. Supported native Edit/RichEdit/Windows Forms controls use owned selection ranges first and guarded clipboard paste only as a secondary transport. Windows Search is handled through UI Automation. Explicit Remote Desktop window classes (`RAIL_WINDOW`, `TscShellContainerClass`, and `TscAxHostClass`) may use the isolated synthetic transport as a last resort; InputKey-tagged injected input is ignored by the low-level hook.

Password and read-only native edit controls are excluded. Queue back-pressure fails open instead of blocking the hook thread. Shutdown is signaled and joined; there is no timer/watchdog transport recovery loop.
