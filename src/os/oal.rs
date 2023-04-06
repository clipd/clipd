use std::ffi::OsString;

use anyhow::Result;

use crate::Args;

pub trait SystemServiceController {
    fn install(&self, arguments: Vec<OsString>) -> Result<()>;
    fn start(&self, arguments: Vec<OsString>) -> Result<()>;
    fn pause(&self) -> Result<()>;
    fn resume(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn status(&self) -> Result<()>;
    fn restart(&self, arguments: Vec<OsString>) -> Result<()>;
    fn uninstall(&self) -> Result<()>;
}

pub trait OsAbstractionLayer: Send + Sync + 'static {
    fn init(&mut self, args: &Args) -> Result<()>;
    fn run_clipd(&self, daemon: bool) -> Result<()>;
    fn service_controller(&self) -> Result<Box<dyn SystemServiceController>>;
}

#[macro_export]
macro_rules! os_stub {
    () => {
        anyhow::bail!("Currently not supported to run on this OS.")
    };
}

pub struct OAL;

impl OAL {
    #[cfg(target_os = "windows")]
    pub fn init(args: &Args) -> Result<Box<dyn OsAbstractionLayer>> {
        let mut oal = Box::new(super::windows::WindowsOAL::default());
        oal.init(args)?;
        Ok(oal)
    }

    #[cfg(target_os = "linux")]
    pub fn init(args: &Args) -> Result<Box<dyn OsAbstractionLayer>> {
        let mut oal = Box::new(super::linux::LinuxOAL::default());
        oal.init(args)?;
        Ok(oal)
    }

    #[cfg(target_os = "macos")]
    pub fn init(_args: &Args) -> Result<Box<dyn OsAbstractionLayer>> {
        os_stub!()
    }
}
