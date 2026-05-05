use std::ops::Range;

use anyhow::{anyhow, bail};

use crate::diff_v2::utils::AtLeastOne;

use super::LineParser;

#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct ChunkHeader {
    pub from_ranges: AtLeastOne<Range<usize>>,
    pub to_range: Range<usize>,
}

pub struct ChunkHeaderParser;

impl<'a> LineParser<'a> for ChunkHeaderParser {
    const NAME: &'static str = "chunk header";

    type Output = ChunkHeader;

    fn parse_line(&self, line: &'a str) -> anyhow::Result<Option<Self::Output>> {
        if !line.starts_with('@') {
            return Ok(None);
        }
        let prefix_end = line.find(|c: char| c != '@').unwrap_or(line.len());

        let prefix = &line[..prefix_end];
        let num_files = prefix.len();

        let mut from_ranges = Vec::new();
        let mut to_range = None;

        let mut rest = &line[prefix_end..];
        for i in 0..num_files {
            let is_parent = (i + 1) != num_files;
            let prefix = if is_parent { '-' } else { '+' };

            let range = rest.trim_start();
            let Some(range) = range.strip_prefix(prefix) else {
                bail!("expected range prefix '{prefix}', got: {line:?}");
            };

            let Some((start, range)) = parse_number(range) else {
                bail!("expected start range, got: {line:?}");
            };
            let Some(range) = range.strip_prefix(',') else {
                bail!("expected range separator ',', got: {line:?}");
            };
            let Some((len, range)) = parse_number(range) else {
                bail!("expected range length, got: {line:?}");
            };

            let Ok(start) = start.parse::<usize>() else {
                unreachable!("All characters are ascii digits in {start:?}");
            };
            let Ok(len) = len.parse::<usize>() else {
                unreachable!("All characters are ascii digits in {len:?}");
            };

            if is_parent {
                from_ranges.push(start..start + len);
            } else {
                to_range = Some(start..start + len);
            }

            rest = range
        }

        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix(prefix) else {
            bail!("expected chunk suffix, got: {rest:?}");
        };
        let _section = rest.trim_start();

        Ok(Some(ChunkHeader {
            from_ranges: from_ranges.try_into()?,
            to_range: to_range.ok_or_else(|| anyhow!("missing to range"))?,
        }))
    }
}

fn parse_number(s: &str) -> Option<(&str, &str)> {
    let rest = s.trim_start_matches(|c: char| c.is_ascii_digit());
    let start = &s[..s.len() - rest.len()];

    if !start.is_empty() {
        Some((start, rest))
    } else {
        None
    }
}

#[test]
fn test_parse_chunk_header() {
    assert_eq!(
        ChunkHeaderParser.parse_line_expected("@@ -0,0 +1,29 @@  "),
        ChunkHeader {
            from_ranges: AtLeastOne::from(0..0),
            to_range: 1..30,
        },
    );
    assert_eq!(
        ChunkHeaderParser
            .parse_line_expected("@@ -161,7 +161,7 @@ fn filter_applicable_comments<'a>("),
        ChunkHeader {
            from_ranges: AtLeastOne::from(161..168),
            to_range: 161..168,
        },
    );
    assert_eq!(
        ChunkHeaderParser.parse_line_expected("@@ -1,13 +1,14 @@"),
        ChunkHeader {
            from_ranges: AtLeastOne::from(1..14),
            to_range: 1..15,
        },
    );
    assert_eq!(
        ChunkHeaderParser.parse_line_expected("@@@ -1,2 -1,2 +1,2 @@@"),
        ChunkHeader {
            from_ranges: range_list([1..3, 1..3]),
            to_range: 1..3,
        },
    );
}

#[cfg(test)]
fn range_list<const N: usize>(ranges: [Range<usize>; N]) -> AtLeastOne<Range<usize>> {
    ranges.into_iter().collect::<Vec<_>>().try_into().unwrap()
}
