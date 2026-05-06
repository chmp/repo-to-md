use std::borrow::Cow;

use anyhow::{Result, ensure};

use super::parser::Parser;

/// A git file mode.
///
/// Example: `100644` from an `old mode`, `new mode`, or `index` line is stored
/// as a `Mode`.
#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct Mode<'a>(pub Cow<'a, str>);

impl<'a> Mode<'a> {
    pub(crate) fn borrowed(mode: &'a str) -> Self {
        Self(Cow::Borrowed(mode))
    }

    pub(crate) fn owned(mode: impl Into<String>) -> Self {
        Self(Cow::Owned(mode.into()))
    }

    pub fn into_static(self) -> Mode<'static> {
        Mode::owned(self.0.into_owned())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ModeParser;

impl<'a> Parser<'a> for ModeParser {
    const NAME: &'static str = "mode";

    type Output = Mode<'a>;

    fn parse(&self, line: &'a str) -> Result<Option<(Self::Output, &'a str)>> {
        let end = line
            .find(|c: char| c.to_digit(8).is_none())
            .unwrap_or(line.len());
        ensure!(end != 0, "no mode");
        let result = (Mode::borrowed(&line[..end]), &line[end..]);
        Ok(Some(result))
    }
}

#[test]
fn parse_mode() {
    assert_eq!(
        ModeParser.parse_expected("100644"),
        Mode::borrowed("100644")
    );
    assert_eq!(ModeParser.parse_expected("0644"), Mode::borrowed("0644"));
    assert_eq!(
        ModeParser.parse_expected("1234567"),
        Mode::borrowed("1234567")
    );
}
