use anyhow::Result;

use super::chunk_header::{ChunkHeader, ChunkHeaderParser};
use super::diff_line::{DiffLine, DiffLineParser};
use super::parser::MultilineParser;

#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct Chunk<'a> {
    pub header: ChunkHeader,
    pub lines: Vec<DiffLine<'a>>,
}

impl<'a> Chunk<'a> {
    pub fn into_static(self) -> Chunk<'static> {
        Chunk {
            header: self.header,
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

        let result = Chunk { header, lines };

        Ok(Some((result, rest)))
    }
}

#[test]
fn chunk_into_static() {
    let lines = ["@@ -1,1 +1,1 @@", " line", "not a diff line"];
    let (chunk, rest) = ChunkParser.parse_lines_required(&lines).unwrap();
    assert_eq!(rest, &["not a diff line"]);

    let _static_chunk: Chunk<'static> = chunk.into_static();
}
