use std::ops::Range;

use anyhow::Result;

use crate::diff::utils::AtLeastOne;

use super::chunk_header::ChunkHeaderParser;
use super::diff_line::{DiffLine, DiffLineParser};
use super::parser::MultilineParser;

/// A unified diff chunk and its parsed lines.
///
/// Example: an `@@ -1,1 +1,2 @@` hunk becomes a `Chunk` with one source range,
/// one target range, and the following context/add/remove lines.
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

    fn parse_lines_at(
        &self,
        lines: &'a [&'a str],
        first_line_number: usize,
    ) -> Result<Option<(Self::Output, &'a [&'a str])>> {
        let Some((header, mut rest)) =
            ChunkHeaderParser.parse_lines_at(lines, first_line_number)?
        else {
            return Ok(None);
        };
        let mut line_number = first_line_number + lines.len() - rest.len();
        let parser = DiffLineParser::new(header.from_ranges.len());
        let mut lines = Vec::new();
        while let Some((head, tail)) = rest.split_first() {
            if head.starts_with('\\') {
                rest = tail;
                line_number += 1;
                continue;
            }

            let Some((line, next_rest)) = parser.parse_lines_at(rest, line_number)? else {
                break;
            };
            line_number += rest.len() - next_rest.len();
            lines.push(line);
            rest = next_rest;
        }

        let result = Chunk {
            from_ranges: header.from_ranges,
            to_range: header.to_range,
            lines,
        };

        Ok(Some((result, rest)))
    }
}
