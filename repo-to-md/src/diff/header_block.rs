use anyhow::{Result, bail, ensure};

use super::diff_header::{DiffHeader, DiffHeaderParser};
use super::extended_header_line::{ExtendedHeaderLine, ExtendedHeaderLineParser};
use super::file_header_line::{NewFileHeaderLineParser, OldFileHeaderLineParser};
use super::parser::MultilineParser;
use super::path::Path;

/// A parsed header block without its following chunks.
///
/// Example: this combines the `diff --git` line, any extended headers, and the
/// `---`/`+++` file header lines.
#[derive(PartialEq, Debug, serde::Serialize)]
pub struct DiffHeaderBlock<'a> {
    pub diff: DiffHeader<'a>,
    pub extended: Vec<ExtendedHeaderLine<'a>>,
    pub old_files: Vec<Option<Path<'a>>>,
    pub new_file: Option<Path<'a>>,
}

impl<'a> DiffHeaderBlock<'a> {
    pub fn into_static(self) -> DiffHeaderBlock<'static> {
        DiffHeaderBlock {
            diff: self.diff.into_static(),
            extended: self
                .extended
                .into_iter()
                .map(ExtendedHeaderLine::into_static)
                .collect(),
            old_files: self
                .old_files
                .into_iter()
                .map(|path| path.map(Path::into_static))
                .collect(),
            new_file: self.new_file.map(Path::into_static),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DiffHeaderBlockParser;

impl<'a> MultilineParser<'a> for DiffHeaderBlockParser {
    const NAME: &'static str = "diff header block";

    type Output = DiffHeaderBlock<'a>;

    fn parse_lines_at(
        &self,
        lines: &'a [&'a str],
        first_line_number: usize,
    ) -> Result<Option<(Self::Output, &'a [&'a str])>> {
        let Some((diff, mut rest)) = DiffHeaderParser.parse_lines_at(lines, first_line_number)?
        else {
            return Ok(None);
        };
        let mut line_number = first_line_number + lines.len() - rest.len();

        let mut extended = Vec::new();
        loop {
            let Some((header, next_rest)) =
                ExtendedHeaderLineParser.parse_lines_at(rest, line_number)?
            else {
                break;
            };
            extended.push(header);
            line_number += rest.len() - next_rest.len();
            rest = next_rest;
        }

        let mut old_files = Vec::new();
        loop {
            let Some((old_file, next_rest)) =
                OldFileHeaderLineParser.parse_lines_at(rest, line_number)?
            else {
                break;
            };
            old_files.push(old_file);
            line_number += rest.len() - next_rest.len();
            rest = next_rest;
        }
        ensure!(
            !old_files.is_empty(),
            "line {line_number}: missing old file header line"
        );

        let Some((new_file, rest)) = NewFileHeaderLineParser.parse_lines_at(rest, line_number)?
        else {
            bail!("line {line_number}: missing new file header line");
        };

        Ok(Some((
            DiffHeaderBlock {
                diff,
                extended,
                old_files,
                new_file,
            },
            rest,
        )))
    }
}

#[test]
fn parse_diff_header_block() {
    use super::hash::Hash;
    use super::index_line::IndexLine;
    use super::mode::Mode;
    use super::path::Path;

    let lines = [
        "diff --git a/src/lib.rs b/src/lib.rs",
        "old mode 100644",
        "new mode 100755",
        "index 7626a52..16399c7",
        "--- a/src/lib.rs",
        "+++ b/src/lib.rs",
        "@@ -1,3 +1,3 @@",
    ];
    let (header, rest) = DiffHeaderBlockParser
        .parse_lines_required_at(&lines, 1)
        .unwrap();
    assert_eq!(
        header,
        DiffHeaderBlock {
            diff: DiffHeader {
                left: Path::borrowed("src/lib.rs"),
                right: Path::borrowed("src/lib.rs"),
            },
            extended: vec![
                ExtendedHeaderLine::OldMode(Mode::borrowed("100644")),
                ExtendedHeaderLine::NewMode(Mode::borrowed("100755")),
                ExtendedHeaderLine::Index(IndexLine {
                    old: Hash::borrowed("7626a52"),
                    new: Hash::borrowed("16399c7"),
                    mode: None,
                }),
            ],
            old_files: vec![Some(Path::borrowed("src/lib.rs"))],
            new_file: Some(Path::borrowed("src/lib.rs")),
        },
    );
    assert_eq!(rest, &["@@ -1,3 +1,3 @@"]);

    let merge_lines = [
        "diff --git a/merged.rs b/merged.rs",
        "--- a/left.rs",
        "--- a/right.rs",
        "+++ b/merged.rs",
        "@@@ -1,2 -1,2 +1,2 @@@",
    ];
    let header = DiffHeaderBlockParser
        .parse_lines_at(&merge_lines, 1)
        .unwrap()
        .unwrap()
        .0;
    assert_eq!(
        header.old_files,
        vec![
            Some(Path::borrowed("left.rs")),
            Some(Path::borrowed("right.rs")),
        ],
    );
    assert_eq!(header.new_file, Some(Path::borrowed("merged.rs")),);
}
