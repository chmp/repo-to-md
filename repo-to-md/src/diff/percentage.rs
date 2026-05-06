use anyhow::{Result, bail, ensure};

use super::parser::Parser;

/// A percentage value from a similarity or dissimilarity index.
///
/// Example: `93%` is stored as `Percentage(93)`.
#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct Percentage(pub u8);

#[derive(Debug, Default, Clone, Copy)]
pub struct PercentageParser;

impl<'a> Parser<'a> for PercentageParser {
    const NAME: &'static str = "percentage";

    type Output = Percentage;

    fn parse(&self, line: &'a str) -> Result<Option<(Self::Output, &'a str)>> {
        let end = line
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(line.len());
        ensure!(end != 0, "no percentage");
        let value = line[..end].parse::<u8>()?;
        ensure!(value <= 100, "percentage exceeds 100");
        let rest = &line[end..];
        let Some(rest) = rest.strip_prefix('%') else {
            bail!("missing percent sign");
        };
        Ok(Some((Percentage(value), rest)))
    }
}

#[test]
fn parse_percentage() {
    assert_eq!(PercentageParser.parse_expected("0%"), Percentage(0));
    assert_eq!(PercentageParser.parse_expected("93%"), Percentage(93));
    assert_eq!(PercentageParser.parse_expected("100%"), Percentage(100));
}
