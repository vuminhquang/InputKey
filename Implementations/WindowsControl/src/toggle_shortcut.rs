use inputkey_windows_settings as settings;
use std::sync::atomic::{AtomicIsize, AtomicU32, AtomicU64, AtomicU8, Ordering};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const TOGGLE_MESSAGE: u32 = WM_APP + 13;

const MOD_CTRL: u8 = 1;
const MOD_ALT: u8 = 2;
const MOD_SHIFT: u8 = 4;
const MOD_WIN: u8 = 8;
const TARGET_SET: u32 = 1 << 31;

const SIDE_LCTRL: u8 = 1;
const SIDE_RCTRL: u8 = 2;
const SIDE_LSHIFT: u8 = 4;
const SIDE_RSHIFT: u8 = 8;

static TARGET: AtomicU32 = AtomicU32::new(0);
static MODIFIERS: AtomicU8 = AtomicU8::new(0);
static PHYSICAL_MODIFIERS: AtomicU8 = AtomicU8::new(0);
static OWNER: AtomicIsize = AtomicIsize::new(0);
static SUPPRESS_0: AtomicU64 = AtomicU64::new(0);
static SUPPRESS_1: AtomicU64 = AtomicU64::new(0);
static SUPPRESS_2: AtomicU64 = AtomicU64::new(0);
static SUPPRESS_3: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    modifiers: u8,
    vk: u16,
}

impl Shortcut {
    fn encode(self) -> u32 {
        TARGET_SET | u32::from(self.modifiers) | (u32::from(self.vk) << 8)
    }

    fn decode(value: u32) -> Option<Self> {
        (value & TARGET_SET != 0).then_some(Self {
            modifiers: (value & 0xff) as u8,
            vk: ((value >> 8) & 0xffff) as u16,
        })
    }
}

fn modifier_bit(vk: u32) -> u8 {
    match vk {
        0x10 | 0xa0 | 0xa1 => MOD_SHIFT,
        0x11 | 0xa2 | 0xa3 => MOD_CTRL,
        0x12 | 0xa4 | 0xa5 => MOD_ALT,
        0x5b | 0x5c => MOD_WIN,
        _ => 0,
    }
}

fn physical_modifier_bit(vk: u32, scan_code: u32, flags: u32) -> u8 {
    match vk {
        x if x == VK_LCONTROL as u32 => SIDE_LCTRL,
        x if x == VK_RCONTROL as u32 => SIDE_RCTRL,
        x if x == VK_CONTROL as u32 => {
            if flags & LLKHF_EXTENDED != 0 {
                SIDE_RCTRL
            } else {
                SIDE_LCTRL
            }
        }
        x if x == VK_LSHIFT as u32 => SIDE_LSHIFT,
        x if x == VK_RSHIFT as u32 => SIDE_RSHIFT,
        x if x == VK_SHIFT as u32 => {
            if scan_code == 0x36 {
                SIDE_RSHIFT
            } else {
                SIDE_LSHIFT
            }
        }
        _ => 0,
    }
}

fn same_side_ctrl_shift(physical: u8) -> bool {
    physical == (SIDE_LCTRL | SIDE_LSHIFT) || physical == (SIDE_RCTRL | SIDE_RSHIFT)
}

fn parse_key(token: &str) -> Option<u16> {
    let upper = token.trim().to_ascii_uppercase();
    if upper.len() == 1 {
        let value = upper.as_bytes()[0];
        if value.is_ascii_alphanumeric() {
            return Some(u16::from(value));
        }
        return match value {
            b';' => Some(0xba),
            b'=' => Some(0xbb),
            b',' => Some(0xbc),
            b'-' => Some(0xbd),
            b'.' => Some(0xbe),
            b'/' => Some(0xbf),
            b'[' => Some(0xdb),
            b']' => Some(0xdd),
            b'\\' => Some(0xdc),
            b'\'' => Some(0xde),
            _ => None,
        };
    }
    match upper.as_str() {
        "SPACE" => Some(VK_SPACE),
        "TAB" => Some(VK_TAB),
        "ENTER" | "RETURN" => Some(VK_RETURN),
        "ESC" | "ESCAPE" => Some(VK_ESCAPE),
        "BACKSPACE" => Some(VK_BACK),
        "DELETE" | "DEL" => Some(VK_DELETE),
        "INSERT" | "INS" => Some(VK_INSERT),
        "HOME" => Some(VK_HOME),
        "END" => Some(VK_END),
        "PAGEUP" | "PAGE_UP" => Some(VK_PRIOR),
        "PAGEDOWN" | "PAGE_DOWN" => Some(VK_NEXT),
        "LEFT" => Some(VK_LEFT),
        "RIGHT" => Some(VK_RIGHT),
        "UP" => Some(VK_UP),
        "DOWN" => Some(VK_DOWN),
        _ if upper.starts_with('F') => upper[1..]
            .parse::<u8>()
            .ok()
            .filter(|value| (1..=12).contains(value))
            .map(|value| 0x6f + u16::from(value)),
        _ => None,
    }
}

fn key_name(vk: u16) -> String {
    match vk {
        0x41..=0x5a | 0x30..=0x39 => char::from_u32(u32::from(vk)).unwrap_or('?').to_string(),
        x if x == VK_SPACE => "Space".into(),
        x if x == VK_TAB => "Tab".into(),
        x if x == VK_RETURN => "Enter".into(),
        x if x == VK_ESCAPE => "Esc".into(),
        x if x == VK_BACK => "Backspace".into(),
        x if x == VK_DELETE => "Delete".into(),
        x if x == VK_INSERT => "Insert".into(),
        x if x == VK_HOME => "Home".into(),
        x if x == VK_END => "End".into(),
        x if x == VK_PRIOR => "PageUp".into(),
        x if x == VK_NEXT => "PageDown".into(),
        x if x == VK_LEFT => "Left".into(),
        x if x == VK_RIGHT => "Right".into(),
        x if x == VK_UP => "Up".into(),
        x if x == VK_DOWN => "Down".into(),
        0xba => ";".into(),
        0xbb => "=".into(),
        0xbc => ",".into(),
        0xbd => "-".into(),
        0xbe => ".".into(),
        0xbf => "/".into(),
        0xdb => "[".into(),
        0xdc => "\\".into(),
        0xdd => "]".into(),
        0xde => "'".into(),
        0x70..=0x7b => format!("F{}", vk - 0x6f),
        _ => format!("VK{vk}"),
    }
}

pub fn parse(value: &str) -> Option<Shortcut> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("off") || value.eq_ignore_ascii_case("none") {
        return None;
    }
    let mut modifiers = 0u8;
    let mut key = None;
    for raw in value.split('+') {
        let token = raw.trim();
        if token.is_empty() {
            return None;
        }
        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= MOD_CTRL,
            "alt" | "option" => modifiers |= MOD_ALT,
            "shift" => modifiers |= MOD_SHIFT,
            "win" | "windows" | "super" | "meta" => modifiers |= MOD_WIN,
            _ => {
                if key.is_some() {
                    return None;
                }
                key = parse_key(token);
                key?;
            }
        }
    }
    if modifiers == 0 {
        return None;
    }
    if modifiers & (MOD_CTRL | MOD_ALT) == (MOD_CTRL | MOD_ALT) {
        return None;
    }
    if key.is_none() && modifiers.count_ones() < 2 {
        return None;
    }
    Some(Shortcut {
        modifiers,
        vk: key.unwrap_or(0),
    })
}

pub fn normalize(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("off")
        || trimmed.eq_ignore_ascii_case("none")
    {
        return Some("Off".into());
    }
    let shortcut = parse(trimmed)?;
    let mut parts = Vec::new();
    if shortcut.modifiers & MOD_CTRL != 0 {
        parts.push("Ctrl".to_string());
    }
    if shortcut.modifiers & MOD_ALT != 0 {
        parts.push("Alt".to_string());
    }
    if shortcut.modifiers & MOD_SHIFT != 0 {
        parts.push("Shift".to_string());
    }
    if shortcut.modifiers & MOD_WIN != 0 {
        parts.push("Win".to_string());
    }
    if shortcut.vk != 0 {
        parts.push(key_name(shortcut.vk));
    }
    Some(parts.join("+"))
}

fn suppression(vk: u32) -> (&'static AtomicU64, u64) {
    let bit = 1u64 << (vk % 64);
    let slot = match vk / 64 {
        0 => &SUPPRESS_0,
        1 => &SUPPRESS_1,
        2 => &SUPPRESS_2,
        _ => &SUPPRESS_3,
    };
    (slot, bit)
}

fn mark_suppressed_up(vk: u32) {
    if vk < 256 {
        let (slot, bit) = suppression(vk);
        slot.fetch_or(bit, Ordering::AcqRel);
    }
}

fn take_suppressed_up(vk: u32) -> bool {
    if vk >= 256 {
        return false;
    }
    let (slot, bit) = suppression(vk);
    slot.fetch_and(!bit, Ordering::AcqRel) & bit != 0
}

pub fn refresh() {
    let settings = settings::load();
    let encoded = parse(&settings.toggle_shortcut)
        .map(Shortcut::encode)
        .unwrap_or(0);
    TARGET.store(encoded, Ordering::Release);
}

unsafe extern "system" fn keyboard_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code < 0 {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }
    let data = unsafe { &*(lp as *const KBDLLHOOKSTRUCT) };
    if data.flags & LLKHF_INJECTED != 0 {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }
    let message = wp as u32;
    let down = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
    let up = matches!(message, WM_KEYUP | WM_SYSKEYUP);
    if !down && !up {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }

    let vk = data.vkCode;
    let modifier = modifier_bit(vk);
    let physical_modifier = physical_modifier_bit(vk, data.scanCode, data.flags);
    let physical_modifiers = if physical_modifier == 0 {
        PHYSICAL_MODIFIERS.load(Ordering::Acquire)
    } else if down {
        PHYSICAL_MODIFIERS.fetch_or(physical_modifier, Ordering::AcqRel) | physical_modifier
    } else {
        PHYSICAL_MODIFIERS.fetch_and(!physical_modifier, Ordering::AcqRel) & !physical_modifier
    };
    let modifiers = if modifier == 0 {
        MODIFIERS.load(Ordering::Acquire)
    } else if down {
        MODIFIERS.fetch_or(modifier, Ordering::AcqRel) | modifier
    } else {
        MODIFIERS.fetch_and(!modifier, Ordering::AcqRel) & !modifier
    };

    if up && take_suppressed_up(vk) {
        return 1;
    }
    let Some(target) = Shortcut::decode(TARGET.load(Ordering::Acquire)) else {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    };
    let matched = if target.vk == 0 {
        let side_matches =
            target.modifiers != (MOD_CTRL | MOD_SHIFT) || same_side_ctrl_shift(physical_modifiers);
        down && modifier != 0 && modifiers == target.modifiers && side_matches
    } else {
        down && modifier == 0 && vk == u32::from(target.vk) && modifiers == target.modifiers
    };
    if !matched {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }

    let owner = OWNER.load(Ordering::Acquire) as HWND;
    if !owner.is_null() {
        unsafe {
            PostMessageW(owner, TOGGLE_MESSAGE, 0, 0);
        }
    }
    mark_suppressed_up(vk);
    1
}

pub struct Hook(HHOOK);

impl Drop for Hook {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                UnhookWindowsHookEx(self.0);
            }
        }
        OWNER.store(0, Ordering::Release);
        TARGET.store(0, Ordering::Release);
        MODIFIERS.store(0, Ordering::Release);
        PHYSICAL_MODIFIERS.store(0, Ordering::Release);
    }
}

pub fn install(owner: HWND) -> Option<Hook> {
    OWNER.store(owner as isize, Ordering::Release);
    refresh();
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0) };
    if hook.is_null() {
        OWNER.store(0, Ordering::Release);
        None
    } else {
        Some(Hook(hook))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_only_ctrl_shift_requires_one_physical_side() {
        assert!(same_side_ctrl_shift(SIDE_LCTRL | SIDE_LSHIFT));
        assert!(same_side_ctrl_shift(SIDE_RCTRL | SIDE_RSHIFT));
        assert!(!same_side_ctrl_shift(SIDE_LCTRL | SIDE_RSHIFT));
        assert!(!same_side_ctrl_shift(SIDE_RCTRL | SIDE_LSHIFT));
        assert!(!same_side_ctrl_shift(
            SIDE_LCTRL | SIDE_LSHIFT | SIDE_RSHIFT
        ));
    }

    #[test]
    fn physical_modifier_mapping_distinguishes_ctrl_and_shift_sides() {
        assert_eq!(
            physical_modifier_bit(VK_LCONTROL as u32, 0x1d, 0),
            SIDE_LCTRL
        );
        assert_eq!(
            physical_modifier_bit(VK_CONTROL as u32, 0x1d, LLKHF_EXTENDED),
            SIDE_RCTRL
        );
        assert_eq!(physical_modifier_bit(VK_SHIFT as u32, 0x2a, 0), SIDE_LSHIFT);
        assert_eq!(physical_modifier_bit(VK_SHIFT as u32, 0x36, 0), SIDE_RSHIFT);
    }

    #[test]
    fn normalizes_supported_shortcuts() {
        assert_eq!(normalize("control + shift"), Some("Ctrl+Shift".into()));
        assert_eq!(normalize("alt+z"), Some("Alt+Z".into()));
        assert_eq!(normalize("Ctrl + Shift + k"), Some("Ctrl+Shift+K".into()));
        assert_eq!(normalize("off"), Some("Off".into()));
        assert_eq!(normalize("Ctrl"), None);
        assert_eq!(normalize("Z"), None);
        assert_eq!(normalize("Ctrl+Alt+K"), None);
    }
}
