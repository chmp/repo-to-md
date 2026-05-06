use anyhow::Result;
use anyhow::bail;

use super::diff_header::{DiffHeader, DiffHeaderParser};
use super::extended_header_line::{ExtendedHeaderLine, ExtendedHeaderLineParser};
use super::file_header_line::{NewFileHeaderLineParser, OldFileHeaderLineParser};
use super::parser::MultilineParser;
use super::path::Path;

/// The complete header block before a file's chunks.
///
/// Example: a header may include `diff --git`, `index`, `---`, and `+++` lines.
/// Combined diffs may contain multiple old file header paths.
#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct DiffFileHeader<'a> {
    pub header: DiffHeader<'a>,
    pub extended_header: Vec<ExtendedHeaderLine<'a>>,
    pub old_files: Vec<Option<Path<'a>>>,
    pub new_file: Option<Path<'a>>,
}

impl<'a> DiffFileHeader<'a> {
    pub fn into_static(self) -> DiffFileHeader<'static> {
        DiffFileHeader {
            header: self.header.into_static(),
            extended_header: self
                .extended_header
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

pub struct DiffFileHeaderParser;

impl<'a> MultilineParser<'a> for DiffFileHeaderParser {
    const NAME: &'static str = "diff file header";

    type Output = DiffFileHeader<'a>;

    fn parse_lines(&self, lines: &'a [&'a str]) -> Result<Option<(Self::Output, &'a [&'a str])>> {
        let Some((header, rest)) = DiffHeaderParser.parse_lines(lines)? else {
            return Ok(None);
        };
        let (extended_header, rest) = ExtendedHeaderLineParser.parse_lines_many(rest)?;
        let (old_files, rest) = OldFileHeaderLineParser.parse_lines_many(rest)?;
        if old_files.is_empty() {
            bail!("missing old file header line");
        }
        let Some((new_file, rest)) = NewFileHeaderLineParser.parse_lines(rest)? else {
            bail!("missing new file header line");
        };

        let result = DiffFileHeader {
            header,
            extended_header,
            old_files,
            new_file,
        };

        Ok(Some((result, rest)))
    }
}

#[test]
fn diff_file_header_into_static() {
    use super::diff_header::DiffHeader;
    use super::extended_header_line::ExtendedHeaderLine;
    use super::mode::Mode;
    use super::path::Path;

    let header = DiffFileHeader {
        header: DiffHeader {
            left: Path::borrowed("old.rs"),
            right: Path::borrowed("new.rs"),
        },
        extended_header: vec![ExtendedHeaderLine::OldMode(Mode::borrowed("100644"))],
        old_files: vec![Some(Path::borrowed("old.rs"))],
        new_file: Some(Path::borrowed("new.rs")),
    };

    let static_header: DiffFileHeader<'static> = header.into_static();
    assert_eq!(
        static_header,
        DiffFileHeader {
            header: DiffHeader {
                left: Path::owned("old.rs"),
                right: Path::owned("new.rs"),
            },
            extended_header: vec![ExtendedHeaderLine::OldMode(Mode::owned("100644"))],
            old_files: vec![Some(Path::owned("old.rs"))],
            new_file: Some(Path::owned("new.rs")),
        }
    );
}
