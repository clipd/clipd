use anyhow::{bail, Context, Result};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{CloseHandle, GetLastError, FALSE, HANDLE},
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{
            CreateFileW, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL, FILE_READ_DATA,
            FILE_SHARE_NONE, FILE_WRITE_DATA, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
        },
        System::Pipes::{CreateNamedPipeW, WaitNamedPipeW, PIPE_TYPE_MESSAGE},
    },
};

use crate::ExpectWithTracing;

#[derive(Debug)]
pub struct Pipe {
    pipe: HANDLE,
}

impl Pipe {
    pub unsafe fn create(name: PCWSTR, sa: Option<SECURITY_ATTRIBUTES>) -> Self {
        let sa = match &sa {
            Some(sa) => Some(sa as *const _),
            None => None,
        };
        let pipe = CreateNamedPipeW(name, PIPE_ACCESS_DUPLEX, PIPE_TYPE_MESSAGE, 1, 0, 0, 0, sa);
        Self { pipe }
    }

    pub unsafe fn connect(name: PCWSTR, timeout: u32) -> Result<Self> {
        if WaitNamedPipeW(name, timeout) == FALSE {
            bail!(
                "WaitPipe {:?} failed: {:?}",
                name.to_string(),
                GetLastError()
            )
        }

        let pipe = CreateFileW(
            name,
            FILE_READ_DATA | FILE_WRITE_DATA,
            FILE_SHARE_NONE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )?;
        if pipe.is_invalid() {
            bail!("Invalid pipe: {:?}", GetLastError())
        }

        Ok(Self { pipe })
    }

    pub fn listen_byte<F>(self, callback: F)
    where
        F: Fn(u8) -> Result<bool> + Send + 'static,
    {
        let pipe = self;
        let thread = std::thread::spawn(move || loop {
            let byte = match pipe.read_byte() {
                Ok(b) => b,
                Err(e) => {
                    log::error!("ReadPipe failed: {:?}", e);
                    break;
                }
            };
            let exit = match callback(byte) {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Handle Pipe error: {:?}", e);
                    continue;
                }
            };
            if exit {
                break;
            }
        });
        let id = thread.thread().id();
        log::debug!("ListenPipe Thread id: {:?}", id);
    }

    pub fn send_byte(&self, byte: u8) {
        unsafe { self.send(&[byte; 1], 1) }
    }

    unsafe fn send(&self, buf: &[u8], length: u32) {
        WriteFile(
            self.pipe,
            Some(buf.as_ptr() as *const _),
            length,
            None,
            None,
        )
        .ok()
        .context(format!("send: {:?}", buf))
        .expectx("WriteFile");
    }

    pub fn read_byte(&self) -> Result<u8> {
        let mut buf = [0u8; 1];
        unsafe { self.read(buf.as_mut_ptr() as *mut _, 1)? };

        Ok(buf[0])
    }

    unsafe fn read(&self, buf: *mut std::ffi::c_void, length: u32) -> Result<()> {
        if ReadFile(self.pipe, Some(buf), length, None, None) == FALSE {
            bail!("ReadFile failed: {:?}", GetLastError())
        };
        Ok(())
    }
}

impl From<&Pipe> for HANDLE {
    fn from(p: &Pipe) -> Self {
        p.pipe
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        log::debug!("ClosePipe: {:?}", self.pipe);
        unsafe { CloseHandle(self.pipe) };
    }
}
