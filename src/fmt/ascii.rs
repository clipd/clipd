use std::collections::VecDeque;

use anyhow::{bail, Result};

use super::{strlen, FormatFeature, Formatter};

#[derive(Default)]
pub struct AsciiFormatter {
    feature: FormatFeature,
}

impl Formatter<u8> for AsciiFormatter {
    fn new(feature: FormatFeature) -> Result<Self> {
        Ok(Self { feature })
    }

    unsafe fn fmt(&self, ptr: *const u8) -> Result<Vec<u8>> {
        let feature = self.feature.expect()?;

        let len = strlen(ptr as *const _);
        if len <= 0 {
            bail!("Invalid str: {:?}", ptr)
        }

        const CR: u8 = 0x0D;
        const LF: u8 = 0x0A;

        let mut deque: VecDeque<u8> = VecDeque::new();
        let mut index = 0;
        while index < len {
            let byte = (ptr as *const u8).add(index).read() as u8;

            if deque.is_empty() {
                if byte == LF && feature.contains(FormatFeature::TRIM_START_LF) {
                    index += 1;
                    continue;
                } else if byte.is_ascii_whitespace()
                    && feature.contains(FormatFeature::TRIM_START_WHITESPACE)
                {
                    index += 1;
                    continue;
                }
            }

            if byte == CR {
                if feature.contains(FormatFeature::TRIM_CR) {
                    index += 1;
                    continue;
                }
            }

            deque.push_back(byte);
            index += 1;
        }

        loop {
            let byte = match deque.back() {
                Some(b) => *b,
                None => break,
            };
            if byte == LF && feature.contains(FormatFeature::TRIM_END_LF) {
                deque.pop_back();
            } else if byte.is_ascii_whitespace()
                && feature.contains(FormatFeature::TRIM_END_WHITESPACE)
            {
                deque.pop_back();
            } else {
                break;
            }
        }
        deque.push_back(0);
        Ok(Vec::from(deque))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{FormatFeature, Formatter},
        AsciiFormatter,
    };

    fn test_fmt<S1, S2>(feature: FormatFeature, source: S1, expect: S2)
    where
        S1: AsRef<str> + std::fmt::Debug,
        S2: AsRef<str> + std::fmt::Debug,
    {
        let formatter =
            AsciiFormatter::new(feature).expect(&format!("Formatter with {:?}", feature));
        let mut c_source = source.as_ref().to_string();
        c_source.push('\0');
        let mut c_expect = expect.as_ref().to_string();
        c_expect.push('\0');
        let fmt_result = unsafe { formatter.fmt(c_source.as_ptr()) }
            .expect(&format!("fmt {:?} with {:?}", source, feature));
        assert_eq!(fmt_result.as_slice(), c_expect.as_bytes());
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
    }
}
