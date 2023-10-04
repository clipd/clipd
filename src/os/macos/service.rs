use std::time::Duration;

use anyhow::Result;

use super::{clipboard::*, fmt::OSXClipboardFormatter};
use crate::fmt::Formatter;

pub struct ClipdService {
    clipboard: OSXClipboard,
    formatter: OSXClipboardFormatter,
}

impl ClipdService {
    pub fn new() -> Result<Self> {
        Ok(Self {
            clipboard: OSXClipboard::new()?,
            formatter: OSXClipboardFormatter::default(),
        })
    }

    pub fn run(&self) -> Result<()> {
        loop {
            if let Err(e) = self.loop_once() {
                log::error!("{:?}", e);
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    fn loop_once(&self) -> Result<()> {
        let clipboard = &self.clipboard;
        let formatter = &self.formatter;
        let text = match clipboard.get_text()? {
            Some(s) => s,
            None => return Ok(()),
        };
        if !formatter.is_need_fmt(&text) {
            return Ok(());
        }
        let fmt_result = formatter.fmt(&text)?;
        log::trace!("{:?}", fmt_result);
        if fmt_result.has_changed() {
            clipboard.set_text(fmt_result.data)?;
        }
        Ok(())
    }
}
