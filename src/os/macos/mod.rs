mod clipboard;
mod fmt;
mod service;

use anyhow::Result;

use super::OsAbstractionLayer;
use crate::os_stub;

#[derive(Debug)]
pub struct MacOAL;

impl OsAbstractionLayer for MacOAL {
    fn init(&mut self, _args: &crate::Args) -> Result<()> {
        Ok(())
    }

    fn run_clipd(&self, daemon: bool) -> Result<()> {
        if daemon {
            os_stub!()
        }
        service::ClipdService::new()?.run()
    }

    fn service_controller(&self) -> Result<Box<dyn super::SystemServiceController>> {
        os_stub!()
    }
}

impl Default for MacOAL {
    fn default() -> Self {
        Self
    }
}
