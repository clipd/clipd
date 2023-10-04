use anyhow::Result;

use crate::fmt::{Formatter, StringFormatter};

use super::clipboard::X11Clipboard;

#[derive(Default)]
pub struct ClipdService;

impl ClipdService {
    pub fn run(&self) -> Result<()> {
        let formatter = StringFormatter::default();
        let clipboard = X11Clipboard::new()?;
        loop {
            let text = match clipboard.wait_utf8_string() {
                Ok(t) => t,
                Err(e) => {
                    log::error!("Get clipboard text failed: {:?}", e);
                    continue;
                }
            };
            let fmt_result = formatter.fmt(&text)?;
            log::trace!("{:?}", fmt_result);
            if fmt_result.has_changed() {
                clipboard.store_utf8_string(fmt_result.data)?;
            }
        }
    }
}
