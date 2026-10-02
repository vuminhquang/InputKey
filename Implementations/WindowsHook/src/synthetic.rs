use windows_sys::Win32::{
    Foundation::HWND,
    UI::{
        Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
            KEYEVENTF_UNICODE, VK_BACK,
        },
        WindowsAndMessaging::{GetAncestor, GetForegroundWindow, IsWindow, GA_ROOT},
    },
};

const INPUTKEY_SYNTHETIC_TAG: usize = 0x494b_5359;

#[derive(Clone, Copy)]
pub struct OwnedSynthetic {
    root: HWND,
}

impl OwnedSynthetic {
    pub fn capture(target: HWND) -> Option<Self> {
        let root = root_window(target);
        if root.is_null() || unsafe { IsWindow(root) } == 0 {
            return None;
        }
        Some(Self { root })
    }

    pub fn target(&self) -> isize {
        self.root as isize
    }

    pub fn replace(&self, old: &str, new: &str) -> Result<(), String> {
        if unsafe { IsWindow(self.root) } == 0
            || root_window(unsafe { GetForegroundWindow() }) != self.root
        {
            return Err("synthetic target is no longer foreground".into());
        }

        let old_chars: Vec<char> = old.chars().collect();
        let new_chars: Vec<char> = new.chars().collect();
        let common = old_chars
            .iter()
            .zip(new_chars.iter())
            .take_while(|(left, right)| left == right)
            .count();

        let delete_count = old_chars.len().saturating_sub(common);
        let insertion: String = new_chars[common..].iter().collect();

        let mut inputs =
            Vec::with_capacity(delete_count * 2 + insertion.encode_utf16().count() * 2);
        for _ in 0..delete_count {
            push_virtual_key(&mut inputs, VK_BACK, false);
            push_virtual_key(&mut inputs, VK_BACK, true);
        }
        for unit in insertion.encode_utf16() {
            push_unicode(&mut inputs, unit, false);
            push_unicode(&mut inputs, unit, true);
        }

        if inputs.is_empty() {
            return Ok(());
        }

        let inserted = unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            )
        };
        if inserted != inputs.len() as u32 {
            return Err(format!(
                "SendInput inserted {inserted}/{} events",
                inputs.len()
            ));
        }
        Ok(())
    }
}

pub fn replay_literal(target: HWND, text: &str) -> bool {
    OwnedSynthetic::capture(target).is_some_and(|owned| owned.replace("", text).is_ok())
}

pub fn root_window(target: HWND) -> HWND {
    if target.is_null() {
        return target;
    }
    let root = unsafe { GetAncestor(target, GA_ROOT) };
    if root.is_null() {
        target
    } else {
        root
    }
}

fn push_virtual_key(out: &mut Vec<INPUT>, vk: u16, key_up: bool) {
    let flags = if key_up { KEYEVENTF_KEYUP } else { 0 };
    out.push(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INPUTKEY_SYNTHETIC_TAG,
            },
        },
    });
}

fn push_unicode(out: &mut Vec<INPUT>, unit: u16, key_up: bool) {
    let flags = KEYEVENTF_UNICODE | if key_up { KEYEVENTF_KEYUP } else { 0 };
    out.push(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INPUTKEY_SYNTHETIC_TAG,
            },
        },
    });
}

#[cfg(test)]
mod tests {
    fn diff(old: &str, new: &str) -> (usize, String) {
        let old_chars: Vec<char> = old.chars().collect();
        let new_chars: Vec<char> = new.chars().collect();
        let common = old_chars
            .iter()
            .zip(new_chars.iter())
            .take_while(|(left, right)| left == right)
            .count();
        (
            old_chars.len().saturating_sub(common),
            new_chars[common..].iter().collect(),
        )
    }

    #[test]
    fn rewrite_uses_unicode_character_boundaries() {
        assert_eq!(diff("tu", "tư"), (1, "ư".into()));
        assert_eq!(diff("ròi", "rồi"), (2, "ồi".into()));
        assert_eq!(diff("đệt", "đệt "), (0, " ".into()));
    }
}
