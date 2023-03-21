mod controller;
mod daemon;
mod error;
mod fmt;
mod icon;
mod mem;
mod mutex;
mod pipe;
mod security;
mod user;
mod window;
use controller::WindowsServiceController;
use error::*;

use std::path::PathBuf;

use anyhow::Result;
use windows::{core::PCWSTR, w};
use windows_service::service::ServiceType;

use super::oal::*;
use crate::{Args, ExpectWithTracing, SubCommand, SERVICE_NAME};

const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

const CLIPD_MUTEX_NAME: &'static str = "Global\\Clipd_InstanceMutex\0";
const PIPE: PCWSTR = w!("\\\\.\\pipe\\clipd");
const MESSAGE_PAUSE: u8 = 0x50;
const MESSAGE_RESUME: u8 = 0x52;
const MESSAGE_EXIT: u8 = 0x44;

const WM_NOTITY_ICON_REBUILD: u32 = 49340; // restart explorer.exe

#[derive(Debug)]
pub struct WindowsOAL {
    service_name: String,
    service_type: ServiceType,
    service_description: String,
    service_uninstall_timeout: u64,
    executable_path: PathBuf,
}

impl OsAbstractionLayer for WindowsOAL {
    fn init(&mut self, args: &Args) -> Result<()> {
        if let Some(SubCommand::Uninstall(args)) = &args.sub {
            self.service_uninstall_timeout = args.timeout;
        }
        Ok(())
    }

    fn run_clipd(&self, daemon: bool) -> Result<()> {
        if daemon {
            daemon::DaemonClipdServiceDispatcher::run(self.service_name.as_str());
            Ok(())
        } else {
            user::UserClipdServiceDispatcher::run(self.service_name.as_str())
        }
    }

    fn service_controller(&self) -> Result<Box<dyn super::oal::SystemServiceController>> {
        Ok(Box::new(WindowsServiceController::new(
            self.service_type,
            self.service_name.clone(),
            self.service_description.clone(),
            self.executable_path.clone(),
            self.service_uninstall_timeout,
        )))
    }
}

impl Default for WindowsOAL {
    fn default() -> Self {
        Self {
            service_name: SERVICE_NAME.into(),
            service_type: SERVICE_TYPE,
            service_description: SERVICE_NAME.into(),
            service_uninstall_timeout: 5,
            executable_path: current_exe_path(),
        }
    }
}

fn current_exe_path() -> PathBuf {
    std::env::current_exe().expectx("current_exe")
}
