use std::{collections::VecDeque, marker::PhantomData};

use anyhow::{bail, Result};

use super::{strlen, wcslen, FormatFeature, Formatter};

#[derive(Default)]
pub struct UnicodeFormatter<T> {
    feature: FormatFeature,
    _mark: PhantomData<T>,
}

impl<T> UnicodeFormatter<T> {
    fn fmt_string(&self, string: String) -> Result<String> {
        let feature = self.feature.expect()?;

        const CR: char = '\x0D';
        const LF: char = '\x0A';

        let mut deque: VecDeque<char> = VecDeque::new();
        for char in string.chars() {
            if deque.is_empty() {
                if char == LF && feature.contains(FormatFeature::TRIM_START_LF) {
                    continue;
                } else if char.is_whitespace()
                    && feature.contains(FormatFeature::TRIM_START_WHITESPACE)
                {
                    continue;
                }
            }

            if char == CR {
                if feature.contains(FormatFeature::TRIM_CR) {
                    continue;
                }
            }

            deque.push_back(char);
        }
        loop {
            let byte = match deque.back() {
                Some(b) => *b,
                None => break,
            };
            if byte == LF && feature.contains(FormatFeature::TRIM_END_LF) {
                deque.pop_back();
            } else if byte.is_whitespace() && feature.contains(FormatFeature::TRIM_END_WHITESPACE) {
                deque.pop_back();
            } else {
                break;
            }
        }
        deque.push_back('\0');
        Ok(String::from_iter(deque))
    }
}

impl Formatter<u8> for UnicodeFormatter<u8> {
    fn new(feature: FormatFeature) -> Result<Self> {
        Ok(Self {
            feature,
            _mark: PhantomData,
        })
    }

    unsafe fn fmt(&self, ptr: *const u8) -> Result<Vec<u8>> {
        self.feature.expect()?;

        let len = strlen(ptr as *const _);
        if len <= 0 {
            bail!("Invalid str: {:?}", ptr)
        }

        let slice = std::slice::from_raw_parts(ptr, len);
        let string = self.fmt_string(String::from_utf8(slice.to_vec())?)?;
        Ok(Vec::from(string))
    }
}

impl Formatter<u16> for UnicodeFormatter<u16> {
    fn new(feature: FormatFeature) -> Result<Self> {
        Ok(Self {
            feature,
            _mark: PhantomData,
        })
    }

    unsafe fn fmt(&self, ptr: *const u16) -> Result<Vec<u16>> {
        self.feature.expect()?;

        let len = wcslen(ptr as *const _);
        if len <= 0 {
            bail!("Invalid str: {:?}", ptr)
        }

        let slice = std::slice::from_raw_parts(ptr, len);
        let string = self.fmt_string(String::from_utf16(slice)?)?;
        Ok(string.encode_utf16().collect::<Vec<u16>>())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{FormatFeature, Formatter, UTF16Formatter, UTF8Formatter};

    fn test_fmt<S1, S2>(feature: FormatFeature, source: S1, expect: S2)
    where
        S1: AsRef<str> + std::fmt::Debug,
        S2: AsRef<str> + std::fmt::Debug,
    {
        // UTF-8
        {
            let formatter =
                UTF8Formatter::new(feature).expect(&format!("Formatter with {:?}", feature));
            let mut c_source = source.as_ref().to_string();
            c_source.push('\0');
            let mut c_expect = expect.as_ref().to_string();
            c_expect.push('\0');
            let fmt_result = unsafe { formatter.fmt(c_source.as_ptr()) }
                .expect(&format!("fmt {:?} with {:?}", source, feature));
            assert_eq!(fmt_result.as_slice(), c_expect.as_bytes());
        }

        // UTF-16
        {
            let formatter =
                UTF16Formatter::new(feature).expect(&format!("Formatter with {:?}", feature));
            let mut c_source = source.as_ref().to_string();
            c_source.push('\0');
            let mut c_expect = expect.as_ref().to_string();
            c_expect.push('\0');
            let ptr = c_source.encode_utf16().collect::<Vec<u16>>().as_ptr();
            let fmt_result = unsafe { formatter.fmt(ptr) }
                .expect(&format!("fmt {:?} with {:?}", source, feature));
            let u16_expect = c_expect.encode_utf16().collect::<Vec<u16>>();
            assert_eq!(fmt_result.as_slice(), u16_expect.as_slice());
        }
    }

    #[test]
    #[should_panic]
    fn empty_feature() {
        test_fmt(FormatFeature::empty(), "foo", "bar");
    }

    #[test]
    #[should_panic]
    fn empty_str() {
        test_fmt(FormatFeature::all(), "", "foo");
    }

    #[test]
    fn it_works() {
        let ws = "\t\n\x0C\r ";
        let ws_nocr = "\t\n\x0C ";

        test_fmt(FormatFeature::TRIM_START_LF, "\nabc", "abc");
        test_fmt(FormatFeature::TRIM_START_LF, "\r\nabc", "\r\nabc");
        test_fmt(FormatFeature::TRIM_START_LF, "\n\n\rabc", "\rabc");
        test_fmt(FormatFeature::TRIM_START_LF, "\nabc\n", "abc\n");

        test_fmt(
            FormatFeature::TRIM_START_WHITESPACE,
            format!("{}abc", ws),
            "abc",
        );
        test_fmt(
            FormatFeature::TRIM_START_WHITESPACE,
            format!("{}{}abc{}{}", ws, ws, ws, ws),
            format!("abc{}{}", ws, ws),
        );

        test_fmt(FormatFeature::TRIM_CR, "\r\nabc\r\n", "\nabc\n");
        test_fmt(FormatFeature::TRIM_CR, "\r\r\na\rb\rc\n\r\r", "\nabc\n");
        test_fmt(FormatFeature::TRIM_CR, "\r\na\r\nb\r\nc\r\n", "\na\nb\nc\n");

        test_fmt(FormatFeature::TRIM_END_LF, "abc\n", "abc");
        test_fmt(FormatFeature::TRIM_END_LF, "abc\n\r", "abc\n\r");
        test_fmt(FormatFeature::TRIM_END_LF, "abc\r\n\n", "abc\r");
        test_fmt(FormatFeature::TRIM_END_LF, "\nabc\n", "\nabc");

        test_fmt(
            FormatFeature::TRIM_END_WHITESPACE,
            format!("abc{}", ws),
            "abc",
        );
        test_fmt(
            FormatFeature::TRIM_END_WHITESPACE,
            format!("{}{}abc{}{}", ws, ws, ws, ws),
            format!("{}{}abc", ws, ws),
        );

        test_fmt(FormatFeature::TRIM_START_END_LF, "\nabc", "abc");
        test_fmt(FormatFeature::TRIM_START_END_LF, "abc\n", "abc");
        test_fmt(FormatFeature::TRIM_START_END_LF, "\nabc\n", "abc");
        test_fmt(
            FormatFeature::TRIM_START_END_LF,
            "\r\nabc\n\r",
            "\r\nabc\n\r",
        );
        test_fmt(
            FormatFeature::TRIM_START_END_LF,
            "\n\n\ra\nb\nc\r\n\n",
            "\ra\nb\nc\r",
        );

        test_fmt(
            FormatFeature::TRIM_START_END_WHITESAPCE,
            format!("{}abc", ws),
            "abc",
        );
        test_fmt(
            FormatFeature::TRIM_START_END_WHITESAPCE,
            format!("abc{}", ws),
            "abc",
        );
        test_fmt(
            FormatFeature::TRIM_START_END_WHITESAPCE,
            format!("{}abc{}", ws, ws),
            "abc",
        );
        test_fmt(
            FormatFeature::TRIM_START_END_WHITESAPCE,
            "  a\rb\tc  ",
            "a\rb\tc",
        );

        test_fmt(
            FormatFeature::TRIM_START_WHITESPACE
                | FormatFeature::TRIM_END_LF
                | FormatFeature::TRIM_CR,
            "\r\n\ta\r\nb\r\nc\r\n ",
            "a\nb\nc\n ",
        );

        test_fmt(
            FormatFeature::all(),
            format!("{}a{}b{}c{}", ws, ws, ws, ws),
            format!("a{}b{}c", ws_nocr, ws_nocr),
        );

        test_fmt(
            FormatFeature::all(),
            format!("{}𝄞{}music{} 音乐𝄞{}", ws, ws, ws, ws),
            format!("𝄞{}music{} 音乐𝄞", ws_nocr, ws_nocr),
        );
    }
}
