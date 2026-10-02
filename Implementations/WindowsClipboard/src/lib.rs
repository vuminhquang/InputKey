#[cfg(windows)]
mod win32 {
    use windows::{
        core::{Error, Result},
        Win32::{
            Foundation::{GlobalFree, HANDLE, HWND},
            System::{
                DataExchange::{
                    CloseClipboard, CountClipboardFormats, EmptyClipboard,
                    GetClipboardSequenceNumber, OpenClipboard, SetClipboardData,
                },
                Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
                Ole::{
                    OleGetClipboard, OleInitialize, OleSetClipboard, OleUninitialize,
                    CF_UNICODETEXT,
                },
            },
            UI::WindowsAndMessaging::{SendMessageW, WM_PASTE},
        },
    };

    struct ClipboardOpen;

    impl ClipboardOpen {
        fn new(owner: Option<HWND>) -> Result<Self> {
            unsafe {
                OpenClipboard(owner)?;
            }
            Ok(Self)
        }
    }

    impl Drop for ClipboardOpen {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseClipboard();
            }
        }
    }

    fn clear_clipboard() -> Result<()> {
        let _open = ClipboardOpen::new(None)?;
        unsafe { EmptyClipboard() }
    }

    fn set_unicode_text(text: &str) -> Result<()> {
        let utf16: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let bytes = utf16.len() * std::mem::size_of::<u16>();
        let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes)? };
        let ptr = unsafe { GlobalLock(memory) };
        if ptr.is_null() {
            unsafe {
                let _ = GlobalFree(Some(memory));
            }
            return Err(Error::from_thread());
        }

        unsafe {
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr.cast::<u16>(), utf16.len());
            let _ = GlobalUnlock(memory);
        }

        let _open = ClipboardOpen::new(None)?;
        unsafe {
            EmptyClipboard()?;
        }
        let set = unsafe { SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(memory.0))) };
        if let Err(error) = set {
            unsafe {
                let _ = GlobalFree(Some(memory));
            }
            return Err(error);
        }
        Ok(())
    }

    pub struct ClipboardPaste {
        ole_initialized: bool,
    }

    impl ClipboardPaste {
        pub fn new() -> Result<Self> {
            unsafe {
                OleInitialize(None)?;
            }
            Ok(Self {
                ole_initialized: true,
            })
        }

        pub fn paste_text(&self, target: isize, text: &str) -> Result<()> {
            if target == 0 {
                return Ok(());
            }

            let formats_before = unsafe { CountClipboardFormats() };
            let original = match unsafe { OleGetClipboard() } {
                Ok(value) => Some(value),
                Err(_error) if formats_before == 0 => None,
                Err(error) => return Err(error),
            };

            set_unicode_text(text)?;
            let ours = unsafe { GetClipboardSequenceNumber() };

            let target = HWND(target as *mut core::ffi::c_void);
            unsafe {
                SendMessageW(target, WM_PASTE, None, None);
            }

            if unsafe { GetClipboardSequenceNumber() } != ours {
                return Ok(());
            }
            match original {
                Some(value) => unsafe { OleSetClipboard(&value) },
                None => clear_clipboard(),
            }
        }
    }

    impl Drop for ClipboardPaste {
        fn drop(&mut self) {
            if self.ole_initialized {
                unsafe {
                    OleUninitialize();
                }
            }
        }
    }
}

#[cfg(windows)]
pub use win32::ClipboardPaste;

#[cfg(not(windows))]
pub struct ClipboardPaste;

#[cfg(not(windows))]
impl ClipboardPaste {
    pub fn new() -> Result<Self, &'static str> {
        Err("Windows clipboard transport is only available on Windows")
    }

    pub fn paste_text(&self, _target: isize, _text: &str) -> Result<(), &'static str> {
        Err("Windows clipboard transport is only available on Windows")
    }
}
