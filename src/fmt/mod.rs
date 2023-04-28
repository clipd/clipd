mod unicode;

use unicode::*;

pub type StringFormatter = UnicodeFormatter;

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

pub trait Formatter<S, R>: Sized + std::fmt::Debug
where
    S: std::fmt::Debug,
{
    fn new(feature: FormatFeature) -> Result<Self>;
    fn new_unchecked(feature: FormatFeature) -> Self {
        Self::new(feature).expect(&format!("Formatter with {:?}", feature))
    }

    fn fmt(&self, text: &S) -> Result<R>;
    fn fmt_unckecked(&self, text: &S) -> R {
        Formatter::fmt(self, text).expect(&format!("{:?} fmt {:?}", self, text))
    }
}
