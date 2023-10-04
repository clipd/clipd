use anyhow::{bail, Context, Result};
use windows::{
    core::PCWSTR,
    Win32::Foundation::*,
    Win32::{
        System::{
            DataExchange::{
                AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData,
                GetClipboardOwner, IsClipboardFormatAvailable, OpenClipboard,
                RemoveClipboardFormatListener, SetClipboardData,
            },
            Memory::{GlobalLock, GlobalUnlock},
            Ole::{CF_HDROP, CF_LOCALE, CF_UNICODETEXT, CLIPBOARD_FORMAT},
        },
        UI::Shell::{DragQueryFileW, HDROP},
    },
};

use crate::{
    fmt::{FormatFeature, FormatResult, Formatter, StringFormatter},
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

        if clipboard.is_available(CF_HDROP) {
            let hdrop = HDROP(clipboard.get_data(CF_HDROP)?.0);
            let count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);
            for i in 0..count {
                let buf_size = DragQueryFileW(hdrop, i, None);
                assert!(buf_size > 0);
                let mut buf = vec![0; buf_size as usize + 1];
                let buf = buf.as_mut();
                assert!(DragQueryFileW(hdrop, i, Some(buf)) == buf_size);
                log::trace!(
                    "Ignore format file content: {:?}",
                    String::from_utf16(buf).unwrap()
                );
            }
            return Ok(());
        }

        if !clipboard.is_available(CF_UNICODETEXT) {
            log::debug!("The clipboard content format is not {:?}", CF_UNICODETEXT);
            return Ok(());
        }

        let text = clipboard.get_data(CF_UNICODETEXT)?;
        let fmt_result = self.utf16_formatter.fmt(&text)?;
        if fmt_result.has_changed() {
            clipboard.set_text(CF_UNICODETEXT, fmt_result.data)?;
        } else {
            log::debug!("No text need formatting");
        }

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

    unsafe fn is_available(&self, format: CLIPBOARD_FORMAT) -> bool {
        IsClipboardFormatAvailable(format.0 as _) == TRUE
    }

    unsafe fn get_data(&self, cf: CLIPBOARD_FORMAT) -> Result<HANDLE> {
        assert!(self.is_available(cf), "Not available format: {:?}", cf);

        let locale = match GetClipboardData(CF_LOCALE.0 as _) {
            Ok(handle) => *(handle.0 as *const u32),
            Err(e) => {
                log::warn!("Get CF_LOCALE failed: {:?}", e);
                0
            }
        };
        log::debug!("GetClipboardData({:?}) with Locale 0x{:04x}", cf, locale);

        Ok(GetClipboardData(cf.0 as _)?)
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

    fn fmt(&self, text: &HANDLE) -> Result<FormatResult<Vec<u16>>> {
        let hmem = text.0;
        let ptr = unsafe { GlobalLock(hmem) };
        let text = PCWSTR::from_raw(ptr as _);
        log::trace!("{:?}", self.inner);
        let text = unsafe { text.to_string()? };
        log::trace!("{:?}", text);
        let fmt_result = self.inner.fmt(&text)?;
        log::trace!("{:?}", fmt_result);
        unsafe { GlobalUnlock(hmem) };
        Ok(fmt_result.map(|s| s.encode_utf16().collect()))
    }
}

impl Default for HANDLE2UTF16Formatter {
    fn default() -> Self {
        Self::new_unchecked(Default::default())
    }
}
