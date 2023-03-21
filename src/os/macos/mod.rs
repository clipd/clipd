use anyhow::Result;

use super::OsAbstractionLayer;
use crate::os_stub;

#[derive(Debug)]
pub struct MacOAL;

impl OsAbstractionLayer for MacOAL {
    fn init(&mut self, _args: &crate::Args) -> Result<()> {
        os_stub!()
    }

    fn run_clipd(&self, _daemon: bool) -> Result<()> {
        os_stub!()
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
