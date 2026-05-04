use anyhow::{Result, ensure};

use super::parser::Parser;

#[derive(Debug, PartialEq)]
pub struct Mode<'a>(pub &'a str);

#[derive(Debug, Default, Clone, Copy)]
pub struct ModeParser;

impl<'a> Parser<'a> for ModeParser {
    type Output = Mode<'a>;

    fn parse(&self, line: &'a str) -> Result<Option<(Self::Output, &'a str)>> {
        let end = line
            .find(|c: char| c.to_digit(8).is_none())
            .unwrap_or(line.len());
        ensure!(end != 0, "no mode");
        let result = (Mode(&line[..end]), &line[end..]);
        Ok(Some(result))
    }
}

#[test]
fn parse_mode() {
    assert_eq!(ModeParser.parse_expected("100644"), Mode("100644"));
    assert_eq!(ModeParser.parse_expected("0644"), Mode("0644"));
    assert_eq!(ModeParser.parse_expected("1234567"), Mode("1234567"));
}
