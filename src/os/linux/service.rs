use anyhow::Result;
use x11rb::protocol::xproto::ConnectionExt;

use crate::fmt::{Formatter, StringFormatter};

#[derive(Default)]
pub struct ClipdService;

impl ClipdService {
    pub fn run(&self) -> Result<()> {
        let formatter = StringFormatter::default();
        let clipboard = x11_clipboard::Clipboard::new()?;
        let atoms = clipboard.getter.atoms.clone();
        loop {
            let text = match clipboard.load_wait(atoms.clipboard, atoms.utf8_string, atoms.property)
            {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Get clipboard text failed: {:?}", e);
                    continue;
                }
            };

            if clipboard
                .setter
                .connection
                .get_selection_owner(atoms.clipboard)?
                .reply()
                .map(|reply| reply.owner == clipboard.setter.window)
                .unwrap_or(false)
            {
                continue;
            }

            let text = match String::from_utf8(text) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Ignore invalid UTF-8 String: {:?}", e);
                    continue;
                }
            };
            let fmt_text = formatter.fmt(&text)?;
            clipboard.store(atoms.clipboard, atoms.utf8_string, fmt_text)?;
        }
    }
}
