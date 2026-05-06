use std::borrow::Cow;

use anyhow::{Result, ensure};

use super::parser::Parser;

/// A commit blob hash from an index line.
///
/// Example: `7626a52` from `index 7626a52..16399c7` is stored as a `Hash`.
#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct Hash<'a>(pub Cow<'a, str>);

impl<'a> Hash<'a> {
    pub(crate) fn borrowed(hash: &'a str) -> Self {
        Self(Cow::Borrowed(hash))
    }

    pub(crate) fn owned(hash: impl Into<String>) -> Self {
        Self(Cow::Owned(hash.into()))
    }

    pub fn into_static(self) -> Hash<'static> {
        Hash::owned(self.0.into_owned())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct HashParser;

impl<'a> Parser<'a> for HashParser {
    const NAME: &'static str = "hash";

    type Output = Hash<'a>;

    fn parse(&self, line: &'a str) -> Result<Option<(Self::Output, &'a str)>> {
        let end = line
            .find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(line.len());
        ensure!(end != 0, "no hash");
        let result = (Hash::borrowed(&line[..end]), &line[end..]);
        Ok(Some(result))
    }
}

#[test]
fn parser_hash() {
    assert_eq!(
        HashParser.parse_expected("7626a52"),
        Hash::borrowed("7626a52")
    );
    assert_eq!(
        HashParser.parse_expected("16399c7"),
        Hash::borrowed("16399c7")
    );
    assert_eq!(
        HashParser.parse_expected("7626a5216399c7"),
        Hash::borrowed("7626a5216399c7")
    );
}
