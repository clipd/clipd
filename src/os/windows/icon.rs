use windows::{
    core::HSTRING,
    Win32::{
        Foundation::HINSTANCE,
        UI::WindowsAndMessaging::{LoadIconW, HICON},
    },
};
use windows_service::service::ServiceState;

use crate::{ExpectWithTracing, Icon};

pub unsafe fn create_tray_icon(hinstance: HINSTANCE, state: ServiceState) -> HICON {
    let icon = match state {
        ServiceState::Paused => Icon::Paused,
        _ => Icon::Running,
    };
    LoadIconW(hinstance, &HSTRING::from(icon.name())).expectx("LoadIcon")
}
