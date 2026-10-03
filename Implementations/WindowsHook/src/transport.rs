use crate::{native, synthetic, uia};
use inputkey_windows_clipboard::ClipboardPaste;
use windows_sys::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        FindWindowExW, GetClassNameW, GetWindowThreadProcessId, IsWindow, HWND_MESSAGE,
    },
};

const TSF_CONTROL_CLASS: &str = "InputKeyTSFControlWindow";

pub enum Capture {
    Ready(OwnedTransport),
    Unsupported,
    Denied,
}

pub enum OwnedTransport {
    Native(native::OwnedRange),
    Automation(uia::OwnedText),
    Synthetic {
        target: synthetic::OwnedSynthetic,
        rendered: String,
    },
}

impl OwnedTransport {
    pub fn target(&self) -> usize {
        match self {
            Self::Native(range) => range.target() as usize,
            Self::Automation(range) => range.target() as usize,
            Self::Synthetic { target, .. } => target.target() as usize,
        }
    }

    pub fn replace(
        &mut self,
        automation: Option<&uia::AutomationText>,
        clipboard: Option<&ClipboardPaste>,
        text: &str,
    ) -> bool {
        match self {
            Self::Automation(owned) => {
                automation.is_some_and(|transport| transport.replace(owned, text).is_ok())
            }
            Self::Synthetic { target, rendered } => {
                if target.replace(rendered, text).is_err() {
                    return false;
                }
                *rendered = text.to_owned();
                true
            }
            Self::Native(range) => {
                if range.replace(text) {
                    return true;
                }
                let Some(clipboard) = clipboard else {
                    return false;
                };
                range.select();
                if clipboard.paste_text(range.target(), text).is_err() {
                    return false;
                }
                let expected = range.expected_end(text);
                let Some((start, end)) = native::current_selection(range.target() as HWND) else {
                    return false;
                };
                if start != expected || end != expected {
                    return false;
                }
                range.update_end(expected);
                true
            }
        }
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

pub fn class_name(hwnd: HWND) -> String {
    let mut buffer = [0u16; 128];
    let len = unsafe { GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..len as usize])
}

fn is_remote_window_class(class: &str) -> bool {
    matches!(
        class.to_ascii_lowercase().as_str(),
        "rail_window" | "tscshellcontainerclass" | "tscaxhostclass"
    )
}

pub fn remote_root(target: HWND) -> Option<HWND> {
    if target.is_null() {
        return None;
    }
    let root = synthetic::root_window(target);
    if is_remote_window_class(&class_name(target)) || is_remote_window_class(&class_name(root)) {
        Some(root)
    } else {
        None
    }
}

pub fn canonical_target(target: HWND) -> HWND {
    remote_root(target).unwrap_or(target)
}

fn is_chromium_surface(target: HWND) -> bool {
    let target_class = class_name(target).to_ascii_lowercase();
    let root_class = class_name(synthetic::root_window(target)).to_ascii_lowercase();
    target_class == "chrome_renderwidgethosthwnd"
        || target_class == "chrome_widgetwin_1"
        || root_class == "chrome_widgetwin_1"
}

fn is_automation_surface_name(class: &str) -> bool {
    let class = class.to_ascii_lowercase();
    class == "windows.ui.core.corewindow"
        || class == "windows.ui.input.inputsite.windowclass"
        || class.contains("xaml")
}

fn is_automation_surface(target: HWND) -> bool {
    is_automation_surface_name(&class_name(target))
}

pub fn thread_has_tsf(target: HWND) -> bool {
    let target_thread = unsafe { GetWindowThreadProcessId(target, std::ptr::null_mut()) };
    if target_thread == 0 {
        return false;
    }

    let class = wide(TSF_CONTROL_CLASS);
    let mut after: HWND = std::ptr::null_mut();
    loop {
        let hwnd = unsafe { FindWindowExW(HWND_MESSAGE, after, class.as_ptr(), std::ptr::null()) };
        if hwnd.is_null() {
            return false;
        }
        let service_thread = unsafe { GetWindowThreadProcessId(hwnd, std::ptr::null_mut()) };
        if service_thread == target_thread {
            return true;
        }
        after = hwnd;
    }
}

/// Cheap hook-thread admission. Rich capability probing stays on the worker thread.
pub fn may_support_text(target: HWND, automation_ready: bool) -> bool {
    if target.is_null() || unsafe { IsWindow(target) } == 0 || thread_has_tsf(target) {
        return false;
    }
    if remote_root(target).is_some() {
        return true;
    }
    let class = class_name(target);
    native::is_edit_class(&class)
        || is_chromium_surface(target)
        || is_automation_surface(target)
        || automation_ready
}

/// Worker-thread capability resolver. It chooses by supported text capability,
/// not by application identity.
pub fn capture(target: HWND, automation: Option<&uia::AutomationText>) -> Capture {
    if target.is_null() || unsafe { IsWindow(target) } == 0 || thread_has_tsf(target) {
        return Capture::Unsupported;
    }

    if remote_root(target).is_some() {
        return synthetic::OwnedSynthetic::capture(target)
            .map(|target| {
                Capture::Ready(OwnedTransport::Synthetic {
                    target,
                    rendered: String::new(),
                })
            })
            .unwrap_or(Capture::Unsupported);
    }

    if native::is_edit_class(&class_name(target)) {
        match native::OwnedRange::capture(target) {
            Ok(range) => return Capture::Ready(OwnedTransport::Native(range)),
            Err(native::CaptureError::Denied) => return Capture::Denied,
            Err(native::CaptureError::Unsupported) => {
                // A native-looking control can still expose a usable UI Automation
                // capability. Continue probing instead of ending the resolver here.
            }
        }
    }

    let mut keyboard_text_surface = false;
    if let Some(automation) = automation {
        let mut process_id = 0u32;
        unsafe {
            GetWindowThreadProcessId(target, &mut process_id);
        }
        match automation.capture_focused(target as isize, process_id) {
            Ok(owned) => return Capture::Ready(OwnedTransport::Automation(owned)),
            Err(uia::CaptureError::Denied) => return Capture::Denied,
            Err(uia::CaptureError::Unsupported) => {}
        }
        match automation.focused_keyboard_text_surface(process_id) {
            Ok(supported) => keyboard_text_surface = supported,
            Err(uia::CaptureError::Denied) => return Capture::Denied,
            Err(uia::CaptureError::Unsupported) => {}
        }
    }

    if keyboard_text_surface || is_chromium_surface(target) {
        return synthetic::OwnedSynthetic::capture(target)
            .map(|target| {
                Capture::Ready(OwnedTransport::Synthetic {
                    target,
                    rendered: String::new(),
                })
            })
            .unwrap_or(Capture::Unsupported);
    }

    Capture::Unsupported
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_remote_classes_are_transport_capabilities() {
        for class in ["RAIL_WINDOW", "TscShellContainerClass", "TscAxHostClass"] {
            assert!(is_remote_window_class(class));
        }
    }

    #[test]
    fn system_input_site_is_admitted_for_worker_capability_probe() {
        assert!(is_automation_surface_name(
            "Windows.UI.Input.InputSite.WindowClass"
        ));
    }
}
