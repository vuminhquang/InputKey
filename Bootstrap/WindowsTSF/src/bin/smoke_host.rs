#[cfg(windows)]
fn main() -> windows::core::Result<()> {
    use inputkey_windows_tsf::activate_text_service;
    use std::io::Write;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows::{
        core::{w, Interface, PCWSTR},
        Win32::{
            Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
            System::Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
                COINIT_APARTMENTTHREADED,
            },
            UI::{
                Input::KeyboardAndMouse::SetFocus,
                TextServices::{CLSID_TF_ThreadMgr, ITfKeystrokeMgr, ITfThreadMgr},
                WindowsAndMessaging::{
                    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
                    GetWindowTextLengthW, GetWindowTextW, PostQuitMessage, RegisterClassW,
                    SendMessageW, ShowWindow, TranslateMessage, ES_AUTOHSCROLL, SW_SHOW,
                    WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_DESTROY, WM_SETFOCUS, WNDCLASSW,
                    WS_BORDER, WS_CHILD, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
                },
            },
        },
    };

    static CHILD: AtomicIsize = AtomicIsize::new(0);
    static CHILD2: AtomicIsize = AtomicIsize::new(0);

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_SETFOCUS => {
                let raw = CHILD.load(Ordering::Acquire);
                if raw != 0 {
                    unsafe {
                        let _ = SetFocus(Some(HWND(raw as *mut core::ffi::c_void)));
                    }
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                let raw = CHILD.load(Ordering::Acquire);
                if raw != 0 {
                    for (label, child_raw) in
                        [("TEXT1", raw), ("TEXT2", CHILD2.load(Ordering::Acquire))]
                    {
                        if child_raw == 0 {
                            continue;
                        }
                        let edit = HWND(child_raw as *mut core::ffi::c_void);
                        let len = unsafe { GetWindowTextLengthW(edit) };
                        let mut buffer = vec![0u16; (len.max(0) as usize) + 1];
                        let copied = unsafe { GetWindowTextW(edit, &mut buffer) }.max(0) as usize;
                        let text = String::from_utf16_lossy(&buffer[..copied]);
                        println!("{label}={text}");
                        println!(
                            "{label}_CODES={}",
                            text.encode_utf16()
                                .map(|value| value.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                    }
                    let _ = std::io::stdout().flush();
                }
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                unsafe {
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
    }
    let thread_mgr: ITfThreadMgr =
        unsafe { CoCreateInstance(&CLSID_TF_ThreadMgr, None, CLSCTX_INPROC_SERVER)? };
    let _client_id = unsafe { thread_mgr.Activate()? };
    activate_text_service()?;

    let module =
        unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(PCWSTR::null())? };
    let instance = HINSTANCE(module.0);
    let class = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        hInstance: instance,
        lpszClassName: w!("InputKeyTSFSmokeHostWindow"),
        ..unsafe { std::mem::zeroed() }
    };
    unsafe {
        RegisterClassW(&class);
    }

    let parent = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("InputKeyTSFSmokeHostWindow"),
            w!("InputKey TSF Smoke Host"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            200,
            200,
            760,
            180,
            None,
            None,
            Some(instance),
            None,
        )?
    };

    let edit_style = WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32);
    let edit = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("EDIT"),
            PCWSTR::null(),
            edit_style,
            16,
            24,
            700,
            48,
            Some(parent),
            None,
            Some(instance),
            None,
        )?
    };
    CHILD.store(edit.0 as isize, Ordering::Release);
    let edit2 = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("EDIT"),
            PCWSTR::null(),
            edit_style,
            16,
            84,
            700,
            48,
            Some(parent),
            None,
            Some(instance),
            None,
        )?
    };
    CHILD2.store(edit2.0 as isize, Ordering::Release);

    unsafe {
        let _ = ShowWindow(parent, SW_SHOW);
        let _ = SetFocus(Some(edit));
    }

    println!(
        "READY HWND={} EDIT1={} EDIT2={}",
        parent.0 as isize, edit.0 as isize, edit2.0 as isize
    );
    std::io::stdout().flush().ok();

    if std::env::args().any(|arg| arg == "--space-regression") {
        let key_mgr: ITfKeystrokeMgr = thread_mgr.cast()?;
        for vk in [0x43usize, 0x55usize, 0x20usize] {
            let test = unsafe { key_mgr.TestKeyDown(WPARAM(vk), LPARAM(0))? };
            let down = unsafe { key_mgr.KeyDown(WPARAM(vk), LPARAM(0))? };
            let _ = unsafe { key_mgr.TestKeyUp(WPARAM(vk), LPARAM(0))? };
            let up = unsafe { key_mgr.KeyUp(WPARAM(vk), LPARAM(0))? };
            println!(
                "VK={vk:02X} TEST={} DOWN={} UP={}",
                test.as_bool(),
                down.as_bool(),
                up.as_bool()
            );
        }
        unsafe {
            SendMessageW(parent, WM_CLOSE, None, None);
        }
    }

    let mut msg = unsafe { std::mem::zeroed() };
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    unsafe {
        let _ = thread_mgr.Deactivate();
        CoUninitialize();
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {}
