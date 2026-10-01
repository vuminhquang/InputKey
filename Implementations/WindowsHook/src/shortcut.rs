#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers(pub u8);

impl Modifiers {
    pub const CTRL: u8 = 1;
    pub const ALT: u8 = 2;
    pub const SHIFT: u8 = 4;
    pub const WIN: u8 = 8;

    pub const fn any(self) -> bool {
        self.0 != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub vk: u16,
    pub modifiers: Modifiers,
}

impl Default for Shortcut {
    fn default() -> Self {
        Self {
            vk: 0x20,
            modifiers: Modifiers(Modifiers::CTRL),
        }
    }
}

impl Shortcut {
    pub const fn matches(self, vk: u16, modifiers: Modifiers) -> bool {
        self.vk == vk && self.modifiers.0 == modifiers.0 && modifiers.any()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordResult {
    Pending,
    Cancelled,
    Recorded(Shortcut),
    Ignored,
}

#[derive(Default)]
pub struct ShortcutRecorder {
    pub recording: bool,
}

fn is_modifier_vk(vk: u16) -> bool {
    matches!(
        vk,
        0x10 | 0x11 | 0x12 | 0x5B | 0x5C | 0xA0 | 0xA1 | 0xA2 | 0xA3 | 0xA4 | 0xA5
    )
}

impl ShortcutRecorder {
    pub fn keydown(&mut self, vk: u16, modifiers: Modifiers) -> RecordResult {
        if !self.recording {
            return RecordResult::Ignored;
        }
        if vk == 0x1B {
            self.recording = false;
            return RecordResult::Cancelled;
        }
        if is_modifier_vk(vk) || !modifiers.any() {
            return RecordResult::Pending;
        }

        self.recording = false;
        RecordResult::Recorded(Shortcut { vk, modifiers })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_exact_match() {
        let shortcut = Shortcut::default();
        assert!(shortcut.matches(0x20, Modifiers(Modifiers::CTRL)));
        assert!(!shortcut.matches(0x20, Modifiers(Modifiers::CTRL | Modifiers::SHIFT)));
    }

    #[test]
    fn recorder_accepts_letter_and_rejects_bare_key() {
        let mut recorder = ShortcutRecorder { recording: true };
        assert_eq!(recorder.keydown(0x41, Modifiers(0)), RecordResult::Pending);
        assert_eq!(
            recorder.keydown(0x41, Modifiers(Modifiers::CTRL | Modifiers::SHIFT)),
            RecordResult::Recorded(Shortcut {
                vk: 0x41,
                modifiers: Modifiers(Modifiers::CTRL | Modifiers::SHIFT)
            })
        );
    }

    #[test]
    fn recorder_waits_on_modifier_and_escape_cancels() {
        let mut recorder = ShortcutRecorder { recording: true };
        assert_eq!(
            recorder.keydown(0x11, Modifiers(Modifiers::CTRL)),
            RecordResult::Pending
        );
        assert_eq!(
            recorder.keydown(0x1B, Modifiers(Modifiers::CTRL)),
            RecordResult::Cancelled
        );
    }
}
