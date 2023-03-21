mod ascii;
mod unicode;

pub use ascii::*;
use unicode::*;

pub type UTF8Formatter = UnicodeFormatter<u8>;
pub type UTF16Formatter = UnicodeFormatter<u16>;

use anyhow::{bail, Result};

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct FormatFeature: u32 {
        const TRIM_START_LF = 1 << 1;
        const TRIM_START_WHITESPACE = Self::TRIM_START_LF.bits() | 1 << 2;
        const TRIM_CR = 1 << 16;
        const TRIM_END_LF = 1 << 30;
        const TRIM_END_WHITESPACE = Self::TRIM_END_LF.bits() | 1 << 31;

        const TRIM_START_END_LF = Self::TRIM_START_LF.bits() | Self::TRIM_END_LF.bits();
        const TRIM_START_END_WHITESAPCE = Self::TRIM_START_WHITESPACE.bits()
            | Self::TRIM_END_WHITESPACE.bits();
        const DEFAULT = Self::TRIM_START_END_WHITESAPCE.bits() | Self::TRIM_CR.bits();
    }
}

impl FormatFeature {
    pub fn unexpect(&self, other: FormatFeature) -> Result<&Self> {
        if self.contains(other) {
            bail!("Unsupported Feature: {:?}", other)
        }
        Ok(self)
    }

    pub fn expect(self) -> Result<Self> {
        if self.is_empty() {
            bail!("FormatFeature is empty")
        }
        Ok(self)
    }
}

impl Default for FormatFeature {
    fn default() -> Self {
        Self::DEFAULT
    }
}

extern "C" {
    /// Provided by libc or compiler_builtins.
    fn strlen(s: *const i8) -> usize;
    fn wcslen(s: *const i32) -> usize;
}

pub trait Formatter<T>: Sized {
    fn new(feature: FormatFeature) -> Result<Self>;

    unsafe fn fmt_c_void(&self, ptr: *const std::ffi::c_void) -> Result<Vec<T>> {
        self.fmt(ptr as *const T)
    }

    unsafe fn fmt(&self, ptr: *const T) -> Result<Vec<T>>;
}
