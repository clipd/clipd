// region: use
use std::{
    ffi::OsString,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use anyhow::Result;
use scopeguard::defer;
use windows::{
    core::{HSTRING, PWSTR},
    w,
    Win32::{
        Foundation::{
            CloseHandle, GetLastError, ERROR_PIPE_CONNECTED, ERROR_SERVICE_ALREADY_RUNNING, FALSE,
        },
        Security::{DuplicateTokenEx, SecurityIdentification, TokenPrimary, TOKEN_ACCESS_MASK},
        System::{
            Pipes::ConnectNamedPipe,
            RemoteDesktop::{WTSGetActiveConsoleSessionId, WTSQueryUserToken},
            Threading::{
                CreateProcessAsUserW, ReleaseMutex, HIGH_PRIORITY_CLASS, PROCESS_INFORMATION,
                STARTF_USESHOWWINDOW, STARTUPINFOW,
            },
        },
    },
};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
    service_dispatcher,
};

use super::{
    current_exe_path, mutex::create_app_mutex, panic_win32_error, pipe::Pipe,
    security::security_attributes, CLIPD_MUTEX_NAME, MESSAGE_EXIT, MESSAGE_PAUSE, MESSAGE_RESUME,
    PIPE, SERVICE_NAME, SERVICE_TYPE,
};
use crate::{os::windows::mem::HandleGuard, ExpectWithTracing};
// endregion: use

pub struct DaemonClipdServiceDispatcher;

impl DaemonClipdServiceDispatcher {
    pub fn run(name: &str) {
        service_dispatcher::start(name, ffi_service_main).expectx("Run daemon service");
    }
}

define_windows_service!(ffi_service_main, main);

fn main(arguments: Vec<OsString>) {
    log::debug!("Service argsuments: {:?}", arguments);
    let sa = unsafe { security_attributes() };
    log::debug!("SecurityAttributes created: {:?}", sa);
    let mutex = match create_app_mutex(CLIPD_MUTEX_NAME, sa.into()) {
        Ok(m) => m,
        Err(e) => {
            log::error!("Error: {:?}", e);
            std::process::exit(ERROR_SERVICE_ALREADY_RUNNING.0 as _)
        }
    };
    log::debug!("AppMutex created: {:?}", mutex);
    defer!(unsafe { 
        log::debug!("Release AppMutex: {:?}", mutex);
        ReleaseMutex(mutex)
    };);

    let name = unsafe { arguments.get_unchecked(0).to_str().unwrap_or(SERVICE_NAME) };
    log::debug!("Run service with name: {}", name);
    let service = DaemonClipdService::new(name, SERVICE_TYPE);
    service.run(current_exe_path());
    log::info!("Service exit");
}

// region: Daemon service
#[derive(Debug)]
struct DaemonClipdService {
    name: String,
    service_type: ServiceType,
    status_handle: ServiceStatusHandle,
    control_rx: Receiver<ServiceControl>,
}

impl DaemonClipdService {
    pub fn new(name: &str, service_type: ServiceType) -> Self {
        let (control_tx, control_rx) = mpsc::channel();
        let event_handler = move |control_event| -> ServiceControlHandlerResult {
            log::info!("Event: {:?}", control_event);
            control_tx.send(control_event).unwrap();
            use ServiceControl::*;
            use ServiceControlHandlerResult::*;
            match control_event {
                Pause | Continue | Stop | Interrogate => NoError,
                _ => NotImplemented,
            }
        };

        let status_handle = service_control_handler::register(name, event_handler)
            .expectx("Register service control handler");

        Self {
            name: name.to_string(),
            service_type,
            status_handle,
            control_rx,
        }
    }

    pub fn run(self, bin: PathBuf) {
        self.update_state(SSP_START_PENDING);
        let mut user_proc = UserProcess::start(bin);
        self.update_state(SSP_RUNNING);

        defer!(self.update_state(SSP_STOPED));

        loop {
            let control = match self.control_rx.recv() {
                Ok(c) => c,
                Err(mpsc::RecvError) => break,
            };
            if self
                .handle_control(control, &mut user_proc)
                .unwrap_or_else(|e| {
                    log::error!("Handle control event error: {:?}", e);
                    false
                })
            {
                break;
            }
        }
        user_proc.exit();
    }

    fn handle_control(&self, control: ServiceControl, user_proc: &mut UserProcess) -> Result<bool> {
        match control {
            ServiceControl::Pause => {
                self.update_state(SSP_PAUSE_PENDING);
                user_proc.pause();
                self.update_state(SSP_PAUSED);
            }
            ServiceControl::Continue => {
                self.update_state(SSP_START_PENDING);
                user_proc.resume();
                self.update_state(SSP_RUNNING);
            }
            ServiceControl::Stop => {
                self.update_state(SSP_STOP_PENDING);
                return Ok(true);
            }
            _ => {}
        };
        Ok(false)
    }

    fn update_state(&self, ssp: ServiceStatePreset) {
        self.status_handle
            .set_service_status(ServiceStatus {
                service_type: self.service_type,
                current_state: ssp.0,
                controls_accepted: ssp.1,
                exit_code: ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: Duration::default(),
                process_id: None,
            })
            .expectx("Set service status");
        log::debug!("Set {} state to: {:?} success", self.name, ssp.0);
    }
}
// endregion: Daemon service

// region: ServiceState and ControlAccept
const SCA_EMPTY: ServiceControlAccept = ServiceControlAccept::empty();
const SCA_STOP: ServiceControlAccept = ServiceControlAccept::STOP;
const SCA_PAUSE_CONTINUE: ServiceControlAccept = ServiceControlAccept::from_bits_truncate(
    SCA_STOP.bits() | ServiceControlAccept::PAUSE_CONTINUE.bits(),
);
struct ServiceStatePreset(ServiceState, ServiceControlAccept);

macro_rules! service_state {
    () => {};
    ($($name:ident = $state:ident $sca:ident),+$(,)?) => {
        $(
            const $name: ServiceStatePreset = ServiceStatePreset(ServiceState::$state, $sca);
        )*
    };
}

service_state!(
    SSP_START_PENDING = StartPending SCA_STOP,
    SSP_RUNNING = Running SCA_PAUSE_CONTINUE,
    SSP_PAUSE_PENDING = PausePending SCA_STOP,
    SSP_PAUSED = Paused SCA_PAUSE_CONTINUE,
    SSP_STOP_PENDING = StopPending SCA_EMPTY,
    SSP_STOPED = Stopped SCA_EMPTY,
);
// endregion: ServiceState and ControlAccept

// region: UserProcess
struct UserProcess {
    pipe: Pipe,
}

impl UserProcess {
    pub fn start(bin: PathBuf) -> Self {
        unsafe { Self::unsafe_start(bin) }
    }

    unsafe fn unsafe_start(bin: PathBuf) -> Self {
        let sa = security_attributes();
        let pipe = Pipe::create(PIPE, sa.into());

        let ptoken = HandleGuard::alloc_zero().expectx("AllocPToken");
        let htoken = HandleGuard::alloc_zero().expectx("AllocHToken");
        let mut ptoken = ptoken.handle();
        let mut htoken = htoken.handle();
        WTSQueryUserToken(WTSGetActiveConsoleSessionId(), &mut ptoken as *mut _)
            .expectx("WTSQueryUserToken");

        DuplicateTokenEx(
            ptoken,
            TOKEN_ACCESS_MASK(0),
            None,
            SecurityIdentification,
            TokenPrimary,
            &mut htoken as *mut _,
        )
        .expectx("DuplicateTokenEx");

        let mut pi = PROCESS_INFORMATION::default();
        let mut si = STARTUPINFOW::default();
        si.lpDesktop = PWSTR::from_raw(w!("winsta0\\default").as_ptr() as *mut _);
        si.dwFlags = STARTF_USESHOWWINDOW;

        let bin = HSTRING::from(bin.as_os_str());
        CreateProcessAsUserW(
            htoken,
            &bin,
            PWSTR::null(),
            None,
            None,
            false,
            HIGH_PRIORITY_CLASS,
            None,
            None,
            &si as *const _,
            &mut pi as *mut _,
        )
        .expectx("CreateProcessAsUser");

        CloseHandle(pi.hProcess);
        CloseHandle(pi.hThread);

        log::info!(
            "Create user process success, pid: {:?} tid: {:?}",
            pi.dwProcessId,
            pi.dwThreadId
        );

        if ConnectNamedPipe(&pipe, None) == FALSE && GetLastError() != ERROR_PIPE_CONNECTED {
            panic_win32_error("ConnectNamedPipe")
        };
        log::debug!("User process connected");

        Self { pipe }
    }

    fn pause(&self) {
        self.pipe.send_byte(MESSAGE_PAUSE)
    }

    fn resume(&self) {
        self.pipe.send_byte(MESSAGE_RESUME)
    }

    fn exit(self) {
        self.pipe.send_byte(MESSAGE_EXIT)
    }
}
// endregion: UserProcess
