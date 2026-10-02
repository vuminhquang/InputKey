use windows_sys::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        GetWindowLongPtrW, IsWindow, SendMessageW, ES_PASSWORD, ES_READONLY, GWL_STYLE,
    },
};

const EM_GETSEL: u32 = 0x00b0;
const EM_SETSEL: u32 = 0x00b1;
const EM_REPLACESEL: u32 = 0x00c2;

#[derive(Clone, Copy)]
pub struct OwnedRange {
    target: HWND,
    start: u32,
    end: u32,
}

impl OwnedRange {
    pub fn target(&self) -> isize {
        self.target as isize
    }

    pub fn capture(target: HWND) -> Option<Self> {
        if target.is_null() || unsafe { IsWindow(target) } == 0 {
            return None;
        }
        let style = unsafe { GetWindowLongPtrW(target, GWL_STYLE) } as u32;
        if style & ES_PASSWORD as u32 != 0 || style & ES_READONLY as u32 != 0 {
            return None;
        }
        let (start, end) = current_selection(target)?;
        Some(Self { target, start, end })
    }

    pub fn replace(&mut self, text: &str) -> bool {
        if unsafe { IsWindow(self.target) } == 0 {
            return false;
        }

        unsafe {
            SendMessageW(
                self.target,
                EM_SETSEL,
                self.start as usize,
                self.end as isize,
            );
        }
        let utf16: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        unsafe {
            SendMessageW(self.target, EM_REPLACESEL, 1, utf16.as_ptr() as isize);
        }

        let expected = self.start + text.encode_utf16().count() as u32;
        let Some((start, end)) = current_selection(self.target) else {
            return false;
        };
        if start != expected || end != expected {
            return false;
        }
        self.end = expected;
        true
    }

    pub fn select(&self) {
        unsafe {
            SendMessageW(
                self.target,
                EM_SETSEL,
                self.start as usize,
                self.end as isize,
            );
        }
    }

    pub fn expected_end(&self, text: &str) -> u32 {
        self.start + text.encode_utf16().count() as u32
    }

    pub fn update_end(&mut self, end: u32) {
        self.end = end;
    }
}

pub fn current_selection(target: HWND) -> Option<(u32, u32)> {
    if target.is_null() || unsafe { IsWindow(target) } == 0 {
        return None;
    }
    let mut start = 0u32;
    let mut end = 0u32;
    unsafe {
        SendMessageW(
            target,
            EM_GETSEL,
            (&mut start as *mut u32) as usize,
            (&mut end as *mut u32) as isize,
        );
    }
    Some((start, end))
}

pub fn is_edit_class(class: &str) -> bool {
    let class = class.to_ascii_lowercase();
    class == "edit" || class.starts_with("richedit") || class.starts_with("windowsforms10.edit")
}
