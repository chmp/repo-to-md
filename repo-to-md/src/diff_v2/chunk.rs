use std::ops::Range;

use anyhow::Result;

use crate::diff_v2::utils::AtLeastOne;

use super::chunk_header::ChunkHeaderParser;
use super::diff_line::{DiffLine, DiffLineParser};
use super::parser::MultilineParser;

#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct Chunk<'a> {
    pub from_ranges: AtLeastOne<Range<usize>>,
    pub to_range: Range<usize>,
    pub lines: Vec<DiffLine<'a>>,
}

impl<'a> Chunk<'a> {
    pub fn into_static(self) -> Chunk<'static> {
        Chunk {
            from_ranges: self.from_ranges,
            to_range: self.to_range,
            lines: self.lines.into_iter().map(DiffLine::into_static).collect(),
        }
    }
}

pub struct ChunkParser;

impl<'a> MultilineParser<'a> for ChunkParser {
    const NAME: &'static str = "chunk";

    type Output = Chunk<'a>;

    fn parse_lines(&self, lines: &'a [&'a str]) -> Result<Option<(Self::Output, &'a [&'a str])>> {
        let Some((header, rest)) = ChunkHeaderParser.parse_lines(lines)? else {
            return Ok(None);
        };
        let (lines, rest) = DiffLineParser::new(header.from_ranges.len()).parse_lines_many(rest)?;

        let result = Chunk {
            from_ranges: header.from_ranges,
            to_range: header.to_range,
            lines,
        };

        Ok(Some((result, rest)))
    }
}
