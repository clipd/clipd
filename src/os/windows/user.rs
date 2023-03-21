// region: use
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{bail, Result};
use once_cell::sync::OnceCell;
use scopeguard::defer;
use windows::{
    core::{HSTRING, PCWSTR},
    w,
    Win32::{
        Foundation::{
            GetLastError, ERROR_FILE_NOT_FOUND, ERROR_SEM_TIMEOUT, HANDLE, HWND, LPARAM, LRESULT,
            NO_ERROR, POINT, WPARAM,
        },
        System::{Console::GetConsoleWindow, Threading::ReleaseMutex},
        UI::{
            Shell::{
                ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD,
                NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW, NOTIFY_ICON_MESSAGE,
            },
            WindowsAndMessaging::{
                CreatePopupMenu, DefWindowProcW, DestroyIcon, DestroyWindow, GetCursorPos,
                InsertMenuW, PostQuitMessage, SendMessageW, SetForegroundWindow, ShowWindow,
                TrackPopupMenu, HMENU, MF_BYPOSITION, MF_STRING, SW_HIDE, SW_SHOWNORMAL,
                TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_LEFTBUTTON, WM_CLIPBOARDUPDATE, WM_COMMAND,
                WM_CREATE, WM_DESTROY, WM_GETMINMAXINFO, WM_NCCALCSIZE, WM_NCCREATE, WM_RBUTTONUP,
                WM_USER, WM_WINDOWPOSCHANGING,
            },
        },
    },
};
use windows_service::service::ServiceState;

use super::{
    fmt::ClipboardFormatter,
    mutex::create_app_mutex,
    window::{self, Window},
    CLIPD_MUTEX_NAME, WM_NOTITY_ICON_REBUILD,
};
use crate::{
    os::windows::{pipe::Pipe, MESSAGE_EXIT, MESSAGE_PAUSE, MESSAGE_RESUME, PIPE},
    ExpectWithTracing,
};
// end region use

type ArcService = Arc<Mutex<UserClipdService>>;
static mut SERVICE: OnceCell<ArcService> = OnceCell::new();

const WM_TRAY_ICON: u32 = WM_USER + 0x54;

pub struct UserClipdServiceDispatcher;

impl UserClipdServiceDispatcher {
    pub fn run(name: &str) -> Result<()> {
        let service = UserClipdService::new(name)?;
        let service = Arc::new(Mutex::new(service));
        let service_mutex = service.clone();
        unsafe { SERVICE.set(service).unwrap() };

        let mut service = service_mutex.lock().unwrap();
        service.run();
        drop(service); // relase mutex

        // Windows Message loop
        unsafe { Window::dispatch_message() }
        Ok(())
    }
}

#[derive(Debug)]
enum ServiceType {
    Normal { mutex: HANDLE },
    ForkedFromSevice { pipe: Option<Pipe> },
}

#[derive(Debug)]
struct UserClipdService {
    service_type: ServiceType,
    name: String,
    window: Window,
    state: ServiceState,
    formatter: ClipboardFormatter,
    retry_show_tray_icon: bool,
}

impl UserClipdService {
    fn clone() -> ArcService {
        unsafe {
            let service = SERVICE.get().expectx("UserService");
            service.clone()
        }
    }

    fn new(name: &str) -> Result<UserClipdService> {
        unsafe { Self::unsafe_new(name) }
    }

    unsafe fn unsafe_new(name: &str) -> Result<UserClipdService> {
        let service_type;
        let service_pipe = Pipe::connect(PIPE, 0);
        if GetLastError() == ERROR_FILE_NOT_FOUND || GetLastError() == ERROR_SEM_TIMEOUT {
            let app_mutex = create_app_mutex(CLIPD_MUTEX_NAME, None)?;
            log::debug!("Service not stated, mutex created: {:?}", app_mutex);
            service_type = ServiceType::Normal { mutex: app_mutex };
        } else {
            let service_pipe = service_pipe?;
            log::debug!("Service stated, pipe: {:?}", service_pipe);
            service_type = ServiceType::ForkedFromSevice {
                pipe: Some(service_pipe),
            };
        }
        let window = unsafe {
            Window::create(
                PCWSTR::from_raw(HSTRING::from(name).as_ptr()),
                window::style::invisible(),
                Some(wnd_proc),
            )
        };
        let fmt = ClipboardFormatter::new(window.hwnd);

        Ok(Self {
            service_type,
            name: name.to_owned(),
            window,
            formatter: fmt,
            state: ServiceState::StartPending,
            retry_show_tray_icon: false,
        })
    }

    fn run(&mut self) {
        unsafe { self.unsafe_run() }
    }

    unsafe fn unsafe_run(&mut self) {
        ShowWindow(GetConsoleWindow(), SW_HIDE);
        if let Err(e) = GetLastError().ok() {
            log::warn!("Hide console window failed: {:?}", e);
        }

        let hwnd = self.window.hwnd;
        if let Err(e) = ctrlc::set_handler(move || {
            log::debug!("Exit by ctrlc");
            let ret = SendMessageW(hwnd, WM_DESTROY, WPARAM::default(), LPARAM::default());
            if GetLastError() != NO_ERROR || ret.0 != 0 {
                log::error!("SendMessage failed: {:?}, ret: {:?}", GetLastError(), ret);
            }
        }) {
            log::warn!("Set ctrlc handler failed: {:?}", e);
        }
        self.state = ServiceState::Running;
        self.retry_show_tray_icon = self.show_tray_icon().is_err();

        match &mut self.service_type {
            ServiceType::ForkedFromSevice { pipe } => {
                let pipe = pipe.take().expectx("Take service pipe");
                Self::listen_pipe(pipe);
            }
            _ => {}
        };
    }

    unsafe fn listen_pipe(pipe: Pipe) {
        log::debug!("Start listen pipe: {:?}", pipe);
        let service = UserClipdService::clone();
        pipe.listen_byte(move |byte| {
            log::trace!("Pipe command: {:x}", byte);
            let mut service = service.lock().unwrap();
            match byte {
                MESSAGE_PAUSE => {
                    service.state = ServiceState::Paused;
                    service.update_tray_icon();
                    drop(service);
                    return Ok(false);
                }
                MESSAGE_RESUME => {
                    service.state = ServiceState::Running;
                    service.update_tray_icon();
                    drop(service);
                    return Ok(false);
                }
                MESSAGE_EXIT => {
                    service.state = ServiceState::Stopped;
                }
                _ => {
                    bail!("Unknown message {:?}", byte);
                }
            };

            let hwnd = service.window.hwnd;
            drop(service);
            let ret = SendMessageW(hwnd, WM_DESTROY, WPARAM::default(), LPARAM::default());
            if GetLastError() != NO_ERROR || ret.0 != 0 {
                bail!("SendMessage failed: {:?}, ret: {:?}", GetLastError(), ret)
            }
            Ok(true)
        });
    }

    fn show_tray_icon(&self) -> Result<()> {
        unsafe { self.notify_tray_icon(NIM_ADD) }
    }

    fn update_tray_icon(&self) {
        unsafe { self.notify_tray_icon(NIM_MODIFY) }.expectx("UpdateTrayIcon")
    }

    fn delete_tray_icon(&self) {
        unsafe { self.notify_tray_icon(NIM_DELETE) }.expectx("DeleteTrayIcon")
    }

    unsafe fn notify_tray_icon(&self, nim: NOTIFY_ICON_MESSAGE) -> Result<()> {
        let mut data = NOTIFYICONDATAW::default();
        data.hWnd = self.window.hwnd;
        match nim {
            NIM_DELETE => {}
            _ => {
                data.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE;
                data.hIcon = super::icon::create_tray_icon(self.window.hinstance, self.state);
                data.szTip = *(format!("{} {:?}", self.name, self.state)
                    .encode_utf16()
                    .collect::<Vec<u16>>()
                    .as_ptr() as *const _);
                data.uCallbackMessage = WM_TRAY_ICON;
            }
        }

        defer!(if !data.hIcon.is_invalid() {
            log::trace!("DestroyIcon: {:?}", data.hIcon);
            DestroyIcon(data.hIcon);
        });

        Ok(Shell_NotifyIconW(nim, &data as *const _).ok()?)
    }
}

impl Drop for UserClipdService {
    fn drop(&mut self) {
        if let ServiceType::Normal { mutex } = self.service_type {
            unsafe { ReleaseMutex(mutex) };
        }
    }
}

unsafe extern "system" fn wnd_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let service = match SERVICE.get() {
        Some(s) => s,
        None => {
            match message {
                WM_GETMINMAXINFO | WM_NCCREATE | WM_NCCALCSIZE | WM_CREATE => {}
                _ => {
                    log::warn!("Service not init: {}", message);
                }
            }
            return DefWindowProcW(window, message, wparam, lparam);
        }
    };
    let service = service.lock().unwrap();

    match message {
        WM_CREATE => {
            log::debug!("WM_CREATE");
        }
        WM_DESTROY => {
            log::debug!("WM_DESTROY");
            service.formatter.destroy();
            service.delete_tray_icon();
            drop(service);

            PostQuitMessage(0);
        }
        WM_CLIPBOARDUPDATE => {
            log::debug!("WM_CLIPBOARDUPDATE");

            if service.state != ServiceState::Running {
                drop(service);
                return LRESULT(0);
            }

            if let Err(e) = service.formatter.fmt() {
                log::error!("WM_CLIPBOARDUPDATE error: {:?}", e);
            }
            drop(service);
        }
        WM_WINDOWPOSCHANGING => {
            if service.retry_show_tray_icon {
                log::debug!("WM_WINDOWPOSCHANGING");
                service.show_tray_icon().ok();
            }
            drop(service);
        }
        WM_NOTITY_ICON_REBUILD => {
            log::debug!("WM_NOTIFY_ICON_REBUILD with state {:?}", service.state);
            service.show_tray_icon().expectx("RebuildTrayIcon");
            drop(service);
        }
        WM_COMMAND => {
            log::debug!("WM_COMMAND");
            handle_tray_ctx_menu_command(service, window, wparam, lparam);
        }
        WM_TRAY_ICON => match lparam.0 as _ {
            WM_RBUTTONUP => {
                log::trace!("WM_RBUTTONUP");
                let mut point = POINT::default();
                GetCursorPos(&mut point as *mut _);
                let popup_menu = CreatePopupMenu().expectx("CreatePopupMenu");
                let menus = match &service.service_type {
                    ServiceType::Normal { mutex: _ } => TrayMenus::normal(),
                    ServiceType::ForkedFromSevice { pipe: _ } => TrayMenus::service(),
                };
                menus.insert(popup_menu, service.state);
                drop(service);

                SetForegroundWindow(window);
                TrackPopupMenu(
                    popup_menu,
                    TPM_LEFTALIGN | TPM_LEFTBUTTON | TPM_BOTTOMALIGN,
                    point.x,
                    point.y,
                    0,
                    window,
                    None,
                );
            }
            _ => drop(service),
        },
        _ => {
            drop(service);
            return DefWindowProcW(window, message, wparam, lparam);
        }
    }
    LRESULT(0)
}

// region: TrayMenu
type MenuId = usize;
const MENU_COMMAND_PAUSE: MenuId = 1;
const MENU_COMMAND_PAUSE_SERVICE: MenuId = 102;
const MENU_COMMAND_RESUME: MenuId = 2;
const MENU_COMMAND_RESUME_SERVICE: MenuId = 103;
const MENU_COMMAND_HELP: MenuId = 3;
const MENU_COMMAND_EXIT: MenuId = 4;
const MENU_COMMAND_EXIT_SERVICE: MenuId = 104;

struct Menu(MenuId, PCWSTR);

struct TrayMenus {
    pause: Menu,
    resume: Menu,
    help: Menu,
    exit: Menu,
}

impl TrayMenus {
    fn normal() -> Self {
        Self {
            pause: Menu(MENU_COMMAND_PAUSE, w!("暂停")),
            resume: Menu(MENU_COMMAND_RESUME, w!("继续")),
            help: Menu(MENU_COMMAND_HELP, w!("帮助")),
            exit: Menu(MENU_COMMAND_EXIT, w!("退出")),
        }
    }

    fn service() -> Self {
        let mut menus = Self::normal();
        menus.pause.0 = MENU_COMMAND_PAUSE_SERVICE;
        menus.resume.0 = MENU_COMMAND_RESUME_SERVICE;
        menus.exit.0 = MENU_COMMAND_EXIT_SERVICE;
        menus
    }

    fn insert(self, hmenu: HMENU, state: ServiceState) {
        let mut menus = vec![];
        match state {
            ServiceState::Paused => {
                menus.push(self.resume);
            }
            _ => {
                menus.push(self.pause);
            }
        }
        menus.push(self.help);
        menus.push(self.exit);
        for (pos, menu) in menus.into_iter().enumerate() {
            unsafe { InsertMenuW(hmenu, pos as _, MF_STRING | MF_BYPOSITION, menu.0, menu.1) };
        }
    }
}

unsafe fn handle_tray_ctx_menu_command(
    mut service: MutexGuard<'_, UserClipdService>,
    window: HWND,
    wparam: WPARAM,
    _lparam: LPARAM,
) {
    let command_id = wparam.0 as _;
    log::trace!("Menu Command: {:?}", command_id);

    let service_name = service.name.clone();
    let runas = move |cmd: &'static str| {
        ShellExecuteW(
            None,
            w!("runas"),
            w!("C:\\Windows\\System32\\sc.exe"),
            &HSTRING::from(format!("{} {}", cmd, service_name)),
            None,
            SW_SHOWNORMAL,
        );
        GetLastError()
            .ok()
            .expectx(format!("ShellExecuteW: {:?}", cmd));
    };

    match command_id {
        MENU_COMMAND_PAUSE_SERVICE => {
            drop(service);
            runas("pause");
        }
        MENU_COMMAND_RESUME_SERVICE => {
            drop(service);
            runas("continue");
        }
        MENU_COMMAND_EXIT_SERVICE => {
            drop(service);
            runas("stop");
        }
        MENU_COMMAND_PAUSE => {
            service.state = ServiceState::Paused;
            service.update_tray_icon();
            drop(service);
        }
        MENU_COMMAND_RESUME => {
            service.state = ServiceState::Running;
            service.update_tray_icon();
            drop(service);
        }
        MENU_COMMAND_HELP => {
            drop(service);
            ShellExecuteW(
                None,
                w!("open"),
                w!("explorer.exe"),
                w!("http://clipd.org"),
                None,
                SW_SHOWNORMAL,
            );
        }
        MENU_COMMAND_EXIT => {
            service.state = ServiceState::Stopped;
            drop(service);
            DestroyWindow(window).expectx("DestroyWindow");
        }
        _ => {
            drop(service);
        }
    }
    GetLastError().ok().expectx("Handle menu command");
}
// endregion: TrayMenu
