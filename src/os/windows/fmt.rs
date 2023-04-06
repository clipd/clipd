use anyhow::{bail, Context, Result};
use windows::{
    core::PCWSTR,
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
    fmt::{FormatFeature, Formatter, StringFormatter},
    os::windows::mem::HandleGuard,
    ExpectWithTracing,
};

#[derive(Debug)]
pub struct ClipboardFormatter {
    utf16_formatter: HANDLE2UTF16Formatter,
    window: HWND,
}

impl ClipboardFormatter {
    pub fn new(window: HWND) -> Self {
        unsafe { AddClipboardFormatListener(window).expectx("AddClipboardFormatListener") };
        Self {
            window,
            utf16_formatter: HANDLE2UTF16Formatter::default(),
        }
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
        let fmt_text = self.utf16_formatter.fmt(&text)?;
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

#[derive(Debug)]
struct HANDLE2UTF16Formatter {
    inner: StringFormatter,
}

impl Formatter<HANDLE, Vec<u16>> for HANDLE2UTF16Formatter {
    fn new(feature: FormatFeature) -> Result<Self> {
        Ok(Self {
            inner: StringFormatter::new(feature)?.ends_with_zero(),
        })
    }

    fn fmt(&self, text: &HANDLE) -> Result<Vec<u16>> {
        let hmem = text.0;
        let ptr = unsafe { GlobalLock(hmem) };
        let text = PCWSTR::from_raw(ptr as _);
        unsafe {
            log::trace!("{:?}", text.to_string());
        }
        log::trace!("{:?}", self.inner);
        let text = unsafe { text.to_string()? };
        let fmt_text = self.inner.fmt(&text)?;
        log::trace!("{:?}", fmt_text);
        unsafe { GlobalUnlock(hmem) };
        let b = fmt_text.encode_utf16().collect::<Vec<u16>>();
        log::trace!("{:?}", b);
        Ok(b)
    }
}

impl Default for HANDLE2UTF16Formatter {
    fn default() -> Self {
        Self::new_unchecked(Default::default())
    }
}
