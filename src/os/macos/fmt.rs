use std::cell::RefCell;

use anyhow::Result;

use crate::fmt::{FormatFeature, Formatter, StringFormatter};

#[derive(Debug, Default)]
pub struct OSXClipboardFormatter {
    inner: StringFormatter,
    last: RefCell<Option<String>>,
}

impl Formatter<String, String> for OSXClipboardFormatter {
    fn new(feature: FormatFeature) -> Result<Self> {
        Ok(Self {
            inner: StringFormatter::new(feature)?.ends_with_zero(),
            last: RefCell::new(None),
        })
    }

    fn fmt(&self, text: &String) -> Result<String> {
        let mut last = self.last.borrow_mut();
        let fmt_text = self.inner.fmt(text)?;
        Ok(last.insert(fmt_text).clone())
    }
}

impl OSXClipboardFormatter {
    pub fn is_need_fmt(&self, text: &String) -> bool {
        match self.last.borrow().as_ref() {
            Some(s) => !s.eq(text),
            None => true,
        }
    }
}
