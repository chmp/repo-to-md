use anyhow::Result;

use super::chunk::{Chunk, ChunkParser};
use super::diff_file_header::{DiffFileHeader, DiffFileHeaderParser, is_binary_diff_marker};
use super::diff_header::DiffHeaderParser;
use super::parser::{LineParser, MultilineParser};

/// The parsed diff for a single file.
///
/// Example: one `diff --git a/src/lib.rs b/src/lib.rs` block becomes one
/// `DiffFile` containing its file header and all parsed chunks.
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

    fn parse_lines_at(
        &self,
        lines: &'a [&'a str],
        first_line_number: usize,
    ) -> Result<Option<(Self::Output, &'a [&'a str])>> {
        let Some((header, rest)) = DiffFileHeaderParser.parse_lines_at(lines, first_line_number)?
        else {
            return Ok(None);
        };
        let chunks_first_line = first_line_number + lines.len() - rest.len();
        let is_binary = rest.first().is_some_and(|line| is_binary_diff_marker(line));
        let (chunks, rest) = if is_binary {
            (Vec::new(), skip_binary_diff(rest))
        } else {
            ChunkParser.parse_lines_many_at(rest, chunks_first_line)?
        };

        let result = DiffFile { header, chunks };

        Ok(Some((result, rest)))
    }
}

fn skip_binary_diff<'a>(mut lines: &'a [&'a str]) -> &'a [&'a str] {
    while let Some((line, rest)) = lines.split_first() {
        if matches!(DiffHeaderParser.parse_line(line), Ok(Some(_))) {
            return lines;
        }
        lines = rest;
    }
    lines
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
    let (file, rest) = DiffFileParser.parse_lines_required_at(&lines, 1).unwrap();
    assert_eq!(rest, &["next file"]);

    let _static_file: DiffFile<'static> = file.into_static();
}
