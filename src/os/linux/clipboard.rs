use anyhow::Result;

use x11_clipboard::{error::Error, Atom, Clipboard};
use x11rb::{
    connection::Connection,
    protocol::{
        xfixes,
        xproto::{AtomEnum, ConnectionExt},
        Event,
    },
    rust_connection::ConnectError,
};

pub struct X11Clipboard {
    clipboard: Clipboard,
}

impl X11Clipboard {
    pub fn new() -> Result<Self> {
        let clipboard = Clipboard::new()?;
        log::debug!("atoms: {:?}", clipboard.getter.atoms);
        xfixes::query_version(&clipboard.getter.connection, 5, 0)?;
        Ok(Self { clipboard })
    }

    pub fn wait_utf8_string(&self) -> Result<String> {
        let context = &self.clipboard.getter;
        let atoms = &context.atoms;
        let connection = &context.connection;

        let screen = connection
            .setup()
            .roots
            .get(context.screen)
            .ok_or(Error::XcbConnect(ConnectError::InvalidScreen))?;

        let cookie = xfixes::select_selection_input(
            connection,
            screen.root,
            atoms.clipboard,
            xfixes::SelectionEventMask::SET_SELECTION_OWNER,
        )?;
        let sequence_number = cookie.sequence_number();
        log::trace!("sequence number: {:?}", sequence_number);
        cookie.check()?;

        loop {
            let text = match self.read_utf8_string(sequence_number)? {
                Some(t) => t,
                None => continue,
            };

            connection
                .delete_property(context.window, atoms.property)?
                .check()?;

            if connection
                .get_selection_owner(atoms.clipboard)?
                .reply()
                .map(|reply| reply.owner == context.window)
                .unwrap_or(false)
            {
                continue;
            }

            return Ok(text);
        }
    }

    fn read_utf8_string(&self, sequence_number: u64) -> Result<Option<String>> {
        let context = &self.clipboard.getter;
        let atoms = &context.atoms;
        let connection = &context.connection;

        loop {
            let (event, seq) = connection.wait_for_event_with_sequence()?;
            log::trace!("event({:?}): {:?}", seq, event);
            if seq < sequence_number {
                continue;
            }
            match event {
                Event::XfixesSelectionNotify(event) => {
                    connection
                        .convert_selection(
                            context.window,
                            atoms.clipboard,
                            atoms.utf8_string,
                            atoms.property,
                            event.timestamp,
                        )?
                        .check()?;
                }
                Event::SelectionNotify(event) => {
                    if event.selection != atoms.clipboard {
                        continue;
                    };

                    if event.property == Atom::from(AtomEnum::NONE) {
                        return Ok(None);
                    }

                    let reply = connection
                        .get_property(
                            false,
                            context.window,
                            atoms.property,
                            AtomEnum::NONE,
                            0,
                            u32::MAX,
                        )?
                        .reply()?;

                    let text = if reply.type_ == atoms.utf8_string {
                        Some(String::from_utf8(reply.value)?)
                    } else {
                        let name_reply = connection.get_atom_name(reply.type_)?.reply()?;
                        log::trace!(
                            "Ignore unexpected type: {:?}",
                            String::from_utf8(name_reply.name)
                        );
                        None
                    };
                    return Ok(text);
                }
                _ => (),
            }
        }
    }

    pub fn store_utf8_string(&self, value: String) -> Result<()> {
        let atoms = &self.clipboard.getter.atoms;
        self.clipboard
            .store(atoms.clipboard, atoms.utf8_string, value)?;
        Ok(())
    }
}
