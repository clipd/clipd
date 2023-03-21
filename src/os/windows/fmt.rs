use anyhow::{bail, Context, Result};
use windows::{
    Win32::Foundation::*,
    Win32::System::{
        DataExchange::{
            AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData,
            GetClipboardOwner, IsClipboardFormatAvailable, OpenClipboard,
            RemoveClipboardFormatListener, SetClipboardData,
        },
        Memory::{GlobalLock, GlobalUnlock},
        Ole::{CF_LOCALE, CF_UNICODETEXT, CLIPBOARD_FORMAT},
    },
};

use crate::{
    fmt::{Formatter, UTF16Formatter},
    os::windows::mem::HandleGuard,
    ExpectWithTracing,
};

#[derive(Debug)]
pub struct ClipboardFormatter {
    window: HWND,
}

impl ClipboardFormatter {
    pub fn new(window: HWND) -> Self {
        unsafe { AddClipboardFormatListener(window).expectx("AddClipboardFormatListener") };
        Self { window }
    }

    pub fn destroy(&self) {
        let window = self.window;
        unsafe { RemoveClipboardFormatListener(window).expectx("RemoveClipboardFormatListener") };
    }

    pub unsafe fn fmt(&self) -> Result<()> {
        let owner = GetClipboardOwner();
        if owner.eq(&self.window) {
            return Ok(());
        }

        let clipboard = Clipboard::open(self.window)?;

        let text = clipboard.get_text(CF_UNICODETEXT)?;
        let ptr = GlobalLock(text.0);
        let formatter = UTF16Formatter::default();
        let fmt_text = formatter.fmt_c_void(ptr)?;
        GlobalUnlock(text.0);
        clipboard.set_text(CF_UNICODETEXT, fmt_text)?;

        Ok(())
    }
}

struct Clipboard;

impl Clipboard {
    unsafe fn open(window: HWND) -> Result<Self> {
        if OpenClipboard(window) == TRUE {
            return Ok(Self {});
        }
        // ERROR_ACCESS_DENIED 0x80070005
        log::warn!("OpenCliboard failed: {:?}", GetLastError().ok());
        SetLastError(NO_ERROR);
        if OpenClipboard(window) == TRUE {
            return Ok(Self {});
        }
        GetLastError().ok().context("OpenCliboard")?;
        bail!("OpenCliboard failed")
    }

    unsafe fn get_text(&self, format: CLIPBOARD_FORMAT) -> Result<HANDLE> {
        let format = format.0 as _;
        if IsClipboardFormatAvailable(format) == FALSE {
            bail!("NotAvailable: {:?}", GetLastError());
        }

        let locale = match GetClipboardData(CF_LOCALE.0 as _) {
            Ok(handle) => *(handle.0 as *const u32),
            Err(e) => {
                log::warn!("Get CF_LOCALE failed: {:?}", e);
                0
            }
        };
        log::debug!(
            "GetClipboardData({:?}) with Locale 0x{:04x}",
            format,
            locale
        );

        Ok(GetClipboardData(format)?)
    }

    unsafe fn set_text<T>(&self, format: CLIPBOARD_FORMAT, text: Vec<T>) -> Result<()> {
        let handle = HandleGuard::<T>::alloc_moveable(text.len())?;
        let dst = handle.lock();
        std::ptr::copy_nonoverlapping(text.as_ptr(), dst, text.len());
        self.empty_cliboard()?;
        SetClipboardData(format.0.into(), &handle)?;
        Ok(())
    }

    unsafe fn empty_cliboard(&self) -> Result<()> {
        if EmptyClipboard() == FALSE {
            GetLastError().ok().context("EmptyClipboard")?;
        }
        Ok(())
    }
}

impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe { CloseClipboard() };
    }
}
