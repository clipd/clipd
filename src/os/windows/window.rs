use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::{System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*},
};

use crate::{os::windows::panic_win32_error, ExpectWithTracing};

#[derive(Debug, Clone)]
pub struct Window {
    pub hinstance: HINSTANCE,
    pub hwnd: HWND,
}

impl Window {
    pub unsafe fn create(class: PCWSTR, style: WINDOW_STYLE, wnd_proc: WNDPROC) -> Self {
        let hinstance = GetModuleHandleW(None).expectx("GetModuleHandle");
        debug_assert!(!hinstance.is_invalid());

        let wc = WNDCLASSW {
            lpfnWndProc: wnd_proc,
            hInstance: hinstance,
            lpszClassName: class,
            ..Default::default()
        };

        let atom = RegisterClassW(&wc);
        debug_assert!(atom != 0);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class,
            class,
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            hinstance,
            None,
        );
        if hwnd.0 == 0 {
            panic_win32_error("CreateWindowEx");
        }

        Self { hinstance, hwnd }
    }

    pub unsafe fn dispatch_message() {
        log::info!("start dispatch message");

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).into() {
            DispatchMessageW(&message);
        }
    }
}

pub mod style {
    use windows::Win32::UI::WindowsAndMessaging::{WINDOW_STYLE, WS_OVERLAPPEDWINDOW, WS_VISIBLE};

    #[allow(dead_code)]
    pub fn invisible() -> WINDOW_STYLE {
        return WINDOW_STYLE::default();
    }

    #[allow(dead_code)]
    pub fn visible() -> WINDOW_STYLE {
        return WINDOW_STYLE::default() | WS_VISIBLE | WS_OVERLAPPEDWINDOW;
    }
}
