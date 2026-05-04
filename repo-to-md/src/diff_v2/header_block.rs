use anyhow::{Result, bail, ensure};

use super::diff_header::{DiffHeader, DiffHeaderParser};
use super::extended_header_line::{ExtendedHeaderLine, ExtendedHeaderLineParser};
use super::file_header_line::{
    NewFileHeaderLine, NewFileHeaderLineParser, OldFileHeaderLine, OldFileHeaderLineParser,
};
use super::parser::{LineParser, MultilineParser};

#[derive(PartialEq, Debug)]
pub struct DiffHeaderBlock<'a> {
    pub diff: DiffHeader<'a>,
    pub extended: Vec<ExtendedHeaderLine<'a>>,
    pub old_files: Vec<OldFileHeaderLine<'a>>,
    pub new_file: NewFileHeaderLine<'a>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DiffHeaderBlockParser;

impl<'a> MultilineParser<'a> for DiffHeaderBlockParser {
    type Output = DiffHeaderBlock<'a>;

    fn parse_lines(&self, lines: &'a [&'a str]) -> Result<Option<(Self::Output, &'a [&'a str])>> {
        let Some((line, rest)) = lines.split_first() else {
            return Ok(None);
        };
        let Some(diff) = DiffHeaderParser.parse_line(line)? else {
            return Ok(None);
        };
        let mut lines = rest;

        let mut extended = Vec::new();
        while let Some((line, rest)) = lines.split_first() {
            let Some(header) = ExtendedHeaderLineParser.parse_line(line)? else {
                break;
            };
            extended.push(header);
            lines = rest;
        }

        let mut old_files = Vec::new();
        while let Some((line, rest)) = lines.split_first() {
            let Some(old_file) = OldFileHeaderLineParser.parse_line(line)? else {
                break;
            };
            old_files.push(old_file);
            lines = rest;
        }
        ensure!(!old_files.is_empty(), "missing old file header line");

        let Some((line, rest)) = lines.split_first() else {
            bail!("missing new file header line");
        };
        let new_file = NewFileHeaderLineParser.parse_line_required(line)?;

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
    let (header, rest) = DiffHeaderBlockParser.parse_lines_required(&lines).unwrap();
    assert_eq!(
        header,
        DiffHeaderBlock {
            diff: DiffHeader {
                left: Path::borrowed("src/lib.rs"),
                right: Path::borrowed("src/lib.rs"),
            },
            extended: vec![
                ExtendedHeaderLine::OldMode(Mode("100644")),
                ExtendedHeaderLine::NewMode(Mode("100755")),
                ExtendedHeaderLine::Index(IndexLine {
                    old: Hash("7626a52"),
                    new: Hash("16399c7"),
                    mode: None,
                }),
            ],
            old_files: vec![OldFileHeaderLine {
                path: Some(Path::borrowed("src/lib.rs")),
            }],
            new_file: NewFileHeaderLine {
                path: Some(Path::borrowed("src/lib.rs")),
            },
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
    let header = DiffHeaderBlockParser.parse_lines_expected(&merge_lines);
    assert_eq!(
        header.old_files,
        vec![
            OldFileHeaderLine {
                path: Some(Path::borrowed("left.rs")),
            },
            OldFileHeaderLine {
                path: Some(Path::borrowed("right.rs")),
            },
        ],
    );
    assert_eq!(
        header.new_file,
        NewFileHeaderLine {
            path: Some(Path::borrowed("merged.rs")),
        },
    );
}
