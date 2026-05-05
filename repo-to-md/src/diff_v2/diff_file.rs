use anyhow::Result;

use super::chunk::{Chunk, ChunkParser};
use super::diff_file_header::{DiffFileHeader, DiffFileHeaderParser};
use super::parser::MultilineParser;

#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct DiffFile<'a> {
    pub header: DiffFileHeader<'a>,
    pub chunks: Vec<Chunk<'a>>,
}

impl<'a> DiffFile<'a> {
    pub fn into_static(self) -> DiffFile<'static> {
        DiffFile {
            header: self.header.into_static(),
            chunks: self.chunks.into_iter().map(Chunk::into_static).collect(),
        }
    }
}

pub struct DiffFileParser;

impl<'a> MultilineParser<'a> for DiffFileParser {
    const NAME: &'static str = "diff file";

    type Output = DiffFile<'a>;

    fn parse_lines(&self, lines: &'a [&'a str]) -> Result<Option<(Self::Output, &'a [&'a str])>> {
        let Some((header, rest)) = DiffFileHeaderParser.parse_lines(lines)? else {
            return Ok(None);
        };
        let (chunks, rest) = ChunkParser.parse_lines_many(rest)?;

        let result = DiffFile { header, chunks };

        Ok(Some((result, rest)))
    }
}

#[test]
fn diff_file_into_static() {
    let lines = [
        "diff --git a/src/lib.rs b/src/lib.rs",
        "index 7626a52..16399c7 100644",
        "--- a/src/lib.rs",
        "+++ b/src/lib.rs",
        "@@ -1,1 +1,1 @@",
        " line",
        "next file",
    ];
    let (file, rest) = DiffFileParser.parse_lines_required(&lines).unwrap();
    assert_eq!(rest, &["next file"]);

    let _static_file: DiffFile<'static> = file.into_static();
}
