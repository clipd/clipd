use std::{collections::VecDeque, fmt::Debug};

use anyhow::Result;

use super::{FormatFeature, FormatResult, Formatter};

#[derive(Debug, Default)]
pub struct UnicodeFormatter {
    feature: FormatFeature,
    ends_with_zero: bool,
}

impl UnicodeFormatter {
    pub fn ends_with_zero(mut self) -> Self {
        self.ends_with_zero = true;
        self
    }
}

impl Formatter<String, String> for UnicodeFormatter {
    fn new(feature: FormatFeature) -> Result<Self> {
        Ok(Self {
            feature,
            ends_with_zero: false,
        })
    }

    fn fmt(&self, text: &String) -> Result<FormatResult<String>> {
        let feature = self.feature.expect()?;
        let mut matched_feature = FormatFeature::empty();

        const CR: char = '\x0D';
        const LF: char = '\x0A';

        let mut deque: VecDeque<char> = VecDeque::new();
        for char in text.chars() {
            if char == '\0' {
                break;
            }

            if deque.is_empty() {
                if char == LF && feature.contains(FormatFeature::TRIM_START_LF) {
                    matched_feature = matched_feature.union(FormatFeature::TRIM_START_LF);
                    continue;
                } else if char.is_whitespace()
                    && feature.contains(FormatFeature::TRIM_START_WHITESPACE)
                {
                    matched_feature = matched_feature.union(FormatFeature::TRIM_START_WHITESPACE);
                    continue;
                }
            }

            if char == CR {
                if feature.contains(FormatFeature::TRIM_CR) {
                    matched_feature = matched_feature.union(FormatFeature::TRIM_CR);
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
                matched_feature = matched_feature.union(FormatFeature::TRIM_END_LF);
                deque.pop_back();
            } else if byte.is_whitespace() && feature.contains(FormatFeature::TRIM_END_WHITESPACE) {
                matched_feature = matched_feature.union(FormatFeature::TRIM_END_WHITESPACE);
                deque.pop_back();
            } else {
                break;
            }
        }
        if self.ends_with_zero {
            deque.push_back('\0');
        }
        Ok(FormatResult::new(String::from_iter(deque), matched_feature))
    }
}

#[cfg(test)]
mod tests {
    use super::super::{FormatFeature, Formatter, StringFormatter};

    fn test_fmt<S1, S2>(feature: FormatFeature, source: S1, expect: S2)
    where
        S1: AsRef<str> + std::fmt::Debug,
        S2: AsRef<str> + std::fmt::Debug,
    {
        {
            let formatter = StringFormatter::new_unchecked(feature);
            let source = source.as_ref().to_string();
            let expect = expect.as_ref().to_string();
            assert!(!source.ends_with('\0'));
            assert!(!expect.ends_with('\0'));
            let fmt_result = formatter.fmt_unckecked(&source);
            assert_eq!(fmt_result.data, expect);
            assert_ne!(fmt_result.has_changed(), fmt_result.data.eq(&source))
        }

        {
            let mut source = source.as_ref().to_string();
            let mut expect = expect.as_ref().to_string();
            expect.push('\0');
            let formatter = StringFormatter::new_unchecked(feature).ends_with_zero();
            let fmt_result = formatter.fmt_unckecked(&source);
            assert_eq!(fmt_result.data, expect);
            source.push('\0');
            assert_ne!(fmt_result.has_changed(), fmt_result.data.eq(&source))
        }

        {
            let mut source = source.as_ref().to_string();
            let mut expect = expect.as_ref().to_string();
            source.push('\0');
            expect.push('\0');
            let formatter = StringFormatter::new_unchecked(feature).ends_with_zero();
            let fmt_result = formatter.fmt_unckecked(&source);
            assert_eq!(fmt_result.data, expect);
            assert_ne!(fmt_result.has_changed(), fmt_result.data.eq(&source))
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
