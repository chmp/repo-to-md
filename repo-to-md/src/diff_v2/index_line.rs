use anyhow::{Result, bail};

use super::hash::{Hash, HashParser};
use super::mode::{Mode, ModeParser};
use super::parser::Parser;

#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct IndexLine<'a> {
    pub old: Hash<'a>,
    pub new: Hash<'a>,
    pub mode: Option<Mode<'a>>,
}

impl<'a> IndexLine<'a> {
    pub fn into_static(self) -> IndexLine<'static> {
        IndexLine {
            old: self.old.into_static(),
            new: self.new.into_static(),
            mode: self.mode.map(Mode::into_static),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct IndexLineParser;

impl<'a> Parser<'a> for IndexLineParser {
    const NAME: &'static str = "index line";

    type Output = IndexLine<'a>;

    fn parse(&self, line: &'a str) -> Result<Option<(Self::Output, &'a str)>> {
        let Some((old, line)) = HashParser.parse(line)? else {
            return Ok(None);
        };
        let Some(line) = line.strip_prefix("..") else {
            bail!("missing '..' in index line");
        };
        let (new, line) = HashParser.parse_required(line)?;
        let line = line.trim_start();
        let (mode, rest) = if line.is_empty() {
            (None, line)
        } else {
            let (mode, rest) = ModeParser.parse_required(line)?;
            (Some(mode), rest)
        };

        Ok(Some((IndexLine { old, new, mode }, rest)))
    }
}

#[test]
fn parse_index_line() {
    assert_eq!(
        IndexLineParser.parse_expected("7626a52..16399c7 100644"),
        IndexLine {
            old: Hash::borrowed("7626a52"),
            new: Hash::borrowed("16399c7"),
            mode: Some(Mode::borrowed("100644")),
        },
    );
    assert_eq!(
        IndexLineParser.parse_expected("7626a52..16399c7"),
        IndexLine {
            old: Hash::borrowed("7626a52"),
            new: Hash::borrowed("16399c7"),
            mode: None,
        },
    );
}
