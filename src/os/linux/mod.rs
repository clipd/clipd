mod service;
mod clipboard;

use anyhow::Result;

use super::OsAbstractionLayer;
use crate::os_stub;

#[derive(Debug)]
pub struct LinuxOAL;

impl OsAbstractionLayer for LinuxOAL {
    fn init(&mut self, _args: &crate::Args) -> Result<()> {
        Ok(())
    }

    fn run_clipd(&self, daemon: bool) -> Result<()> {
        if daemon {
            os_stub!()
        }
        service::ClipdService::default().run()
    }

    fn service_controller(&self) -> anyhow::Result<Box<dyn super::SystemServiceController>> {
        os_stub!()
    }
}

impl Default for LinuxOAL {
    fn default() -> Self {
        Self
    }
}
