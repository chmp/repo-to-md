use anyhow::{Result, ensure};

use super::parser::Parser;

#[derive(Debug, PartialEq)]
pub struct Hash<'a>(pub &'a str);

#[derive(Debug, Default, Clone, Copy)]
pub struct HashParser;

impl<'a> Parser<'a> for HashParser {
    type Output = Hash<'a>;

    fn parse(&self, line: &'a str) -> Result<Option<(Self::Output, &'a str)>> {
        let end = line
            .find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(line.len());
        ensure!(end != 0, "no hash");
        let result = (Hash(&line[..end]), &line[end..]);
        Ok(Some(result))
    }
}

#[test]
fn parser_hash() {
    assert_eq!(HashParser.parse_expected("7626a52"), Hash("7626a52"));
    assert_eq!(HashParser.parse_expected("16399c7"), Hash("16399c7"));
    assert_eq!(
        HashParser.parse_expected("7626a5216399c7"),
        Hash("7626a5216399c7")
    );
}
