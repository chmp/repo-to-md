use anyhow::Result;

use super::index_line::{IndexLine, IndexLineParser};
use super::mode::{Mode, ModeParser};
use super::parser::LineParser;
use super::path::{Path, PathParser};
use super::percentage::{Percentage, PercentageParser};

/// A single extended header line from a git patch.
///
/// Example: `rename from old.rs` becomes `ExtendedHeaderLine::RenameFrom`.
#[derive(PartialEq, Debug, Clone, serde::Serialize)]
pub enum ExtendedHeaderLine<'a> {
    OldMode(Mode<'a>),
    NewMode(Mode<'a>),
    DeletedFileMode(Mode<'a>),
    NewFileMode(Mode<'a>),
    CopyFrom(Path<'a>),
    CopyTo(Path<'a>),
    RenameFrom(Path<'a>),
    RenameTo(Path<'a>),
    SimilarityIndex(Percentage),
    DissimilarityIndex(Percentage),
    Index(IndexLine<'a>),
}

impl<'a> ExtendedHeaderLine<'a> {
    pub fn into_static(self) -> ExtendedHeaderLine<'static> {
        match self {
            Self::OldMode(mode) => ExtendedHeaderLine::OldMode(mode.into_static()),
            Self::NewMode(mode) => ExtendedHeaderLine::NewMode(mode.into_static()),
            Self::DeletedFileMode(mode) => ExtendedHeaderLine::DeletedFileMode(mode.into_static()),
            Self::NewFileMode(mode) => ExtendedHeaderLine::NewFileMode(mode.into_static()),
            Self::CopyFrom(path) => ExtendedHeaderLine::CopyFrom(path.into_static()),
            Self::CopyTo(path) => ExtendedHeaderLine::CopyTo(path.into_static()),
            Self::RenameFrom(path) => ExtendedHeaderLine::RenameFrom(path.into_static()),
            Self::RenameTo(path) => ExtendedHeaderLine::RenameTo(path.into_static()),
            Self::SimilarityIndex(index) => ExtendedHeaderLine::SimilarityIndex(index),
            Self::DissimilarityIndex(index) => ExtendedHeaderLine::DissimilarityIndex(index),
            Self::Index(index) => ExtendedHeaderLine::Index(index.into_static()),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ExtendedHeaderLineParser;

impl<'a> LineParser<'a> for ExtendedHeaderLineParser {
    const NAME: &'static str = "extended header line";

    type Output = ExtendedHeaderLine<'a>;

    fn parse_line(&self, line: &'a str) -> Result<Option<Self::Output>> {
        if let Some(line) = strip_extended_header_prefix(line, "old mode")? {
            Ok(Some(ExtendedHeaderLine::OldMode(
                ModeParser.parse_line_required(line)?,
            )))
        } else if let Some(line) = strip_extended_header_prefix(line, "new mode")? {
            Ok(Some(ExtendedHeaderLine::NewMode(
                ModeParser.parse_line_required(line)?,
            )))
        } else if let Some(line) = strip_extended_header_prefix(line, "deleted file mode")? {
            Ok(Some(ExtendedHeaderLine::DeletedFileMode(
                ModeParser.parse_line_required(line)?,
            )))
        } else if let Some(line) = strip_extended_header_prefix(line, "new file mode")? {
            Ok(Some(ExtendedHeaderLine::NewFileMode(
                ModeParser.parse_line_required(line)?,
            )))
        } else if let Some(line) = strip_extended_header_prefix(line, "copy from")? {
            Ok(Some(ExtendedHeaderLine::CopyFrom(
                PathParser.parse_line_required(line)?,
            )))
        } else if let Some(line) = strip_extended_header_prefix(line, "copy to")? {
            Ok(Some(ExtendedHeaderLine::CopyTo(
                PathParser.parse_line_required(line)?,
            )))
        } else if let Some(line) = strip_extended_header_prefix(line, "rename from")? {
            Ok(Some(ExtendedHeaderLine::RenameFrom(
                PathParser.parse_line_required(line)?,
            )))
        } else if let Some(line) = strip_extended_header_prefix(line, "rename to")? {
            Ok(Some(ExtendedHeaderLine::RenameTo(
                PathParser.parse_line_required(line)?,
            )))
        } else if let Some(line) = strip_extended_header_prefix(line, "similarity index")? {
            Ok(Some(ExtendedHeaderLine::SimilarityIndex(
                PercentageParser.parse_line_required(line)?,
            )))
        } else if let Some(line) = strip_extended_header_prefix(line, "dissimilarity index")? {
            Ok(Some(ExtendedHeaderLine::DissimilarityIndex(
                PercentageParser.parse_line_required(line)?,
            )))
        } else if let Some(line) = strip_extended_header_prefix(line, "index")? {
            Ok(Some(ExtendedHeaderLine::Index(
                IndexLineParser.parse_line_required(line)?,
            )))
        } else {
            Ok(None)
        }
    }
}

fn strip_extended_header_prefix<'a>(line: &'a str, prefix: &str) -> Result<Option<&'a str>> {
    let Some(rest) = line.strip_prefix(prefix) else {
        return Ok(None);
    };
    if !rest.starts_with(char::is_whitespace) {
        return Ok(None);
    }
    Ok(Some(rest.trim_start()))
}

#[test]
fn parse_extended_header_line() {
    use super::hash::Hash;

    assert_eq!(
        ExtendedHeaderLineParser.parse_line_expected("old mode 100644"),
        ExtendedHeaderLine::OldMode(Mode::borrowed("100644")),
    );
    assert_eq!(
        ExtendedHeaderLineParser.parse_line_expected("new mode 100755"),
        ExtendedHeaderLine::NewMode(Mode::borrowed("100755")),
    );
    assert_eq!(
        ExtendedHeaderLineParser.parse_line_expected("deleted file mode 100644"),
        ExtendedHeaderLine::DeletedFileMode(Mode::borrowed("100644")),
    );
    assert_eq!(
        ExtendedHeaderLineParser.parse_line_expected("new file mode 100644"),
        ExtendedHeaderLine::NewFileMode(Mode::borrowed("100644")),
    );
    assert_eq!(
        ExtendedHeaderLineParser.parse_line_expected("copy from old path.md"),
        ExtendedHeaderLine::CopyFrom(Path::borrowed("old path.md")),
    );
    assert_eq!(
        ExtendedHeaderLineParser.parse_line_expected("copy to new path.md"),
        ExtendedHeaderLine::CopyTo(Path::borrowed("new path.md")),
    );
    assert_eq!(
        ExtendedHeaderLineParser.parse_line_expected("rename from old-name.rs"),
        ExtendedHeaderLine::RenameFrom(Path::borrowed("old-name.rs")),
    );
    assert_eq!(
        ExtendedHeaderLineParser.parse_line_expected("rename to new-name.rs"),
        ExtendedHeaderLine::RenameTo(Path::borrowed("new-name.rs")),
    );
    assert_eq!(
        ExtendedHeaderLineParser.parse_line_expected("similarity index 93%"),
        ExtendedHeaderLine::SimilarityIndex(Percentage(93)),
    );
    assert_eq!(
        ExtendedHeaderLineParser.parse_line_expected("dissimilarity index 7%"),
        ExtendedHeaderLine::DissimilarityIndex(Percentage(7)),
    );
    assert_eq!(
        ExtendedHeaderLineParser.parse_line_expected("index 7626a52..16399c7 100644"),
        ExtendedHeaderLine::Index(IndexLine {
            old: Hash::borrowed("7626a52"),
            new: Hash::borrowed("16399c7"),
            mode: Some(Mode::borrowed("100644")),
        }),
    );
    assert_eq!(
        ExtendedHeaderLineParser.parse_line_expected("index 7626a52..16399c7"),
        ExtendedHeaderLine::Index(IndexLine {
            old: Hash::borrowed("7626a52"),
            new: Hash::borrowed("16399c7"),
            mode: None,
        }),
    );
    assert_eq!(
        ExtendedHeaderLineParser.parse_line_expected("old mode \t 100644"),
        ExtendedHeaderLine::OldMode(Mode::borrowed("100644")),
    );
    assert_eq!(
        ExtendedHeaderLineParser
            .parse_line_expected("rename from \"tab\\tnewline\\nand-utf8-\\302\\265.rs\""),
        ExtendedHeaderLine::RenameFrom(Path::owned("tab\tnewline\nand-utf8-\u{00b5}.rs")),
    );
}
