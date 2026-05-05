use anyhow::{Result, bail, ensure};

use super::diff_header::parse_quoted_diff_header_path;
use super::parser::LineParser;
use super::path::Path;

#[derive(PartialEq, Debug, Clone, serde::Serialize)]
pub enum FileHeaderLine<'a> {
    // Merge diffs can include multiple old/from file header lines before the
    // new/to file header line.
    Old(Option<Path<'a>>),
    New(Option<Path<'a>>),
}

impl<'a> FileHeaderLine<'a> {
    pub fn into_static(self) -> FileHeaderLine<'static> {
        match self {
            Self::Old(path) => FileHeaderLine::Old(path.map(Path::into_static)),
            Self::New(path) => FileHeaderLine::New(path.map(Path::into_static)),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FileHeaderLineParser;

impl<'a> LineParser<'a> for FileHeaderLineParser {
    const NAME: &'static str = "file header line";

    type Output = FileHeaderLine<'a>;

    fn parse_line(&self, line: &'a str) -> Result<Option<Self::Output>> {
        if let Some(line) = OldFileHeaderLineParser.parse_line(line)? {
            return Ok(Some(FileHeaderLine::Old(line)));
        }
        if let Some(line) = NewFileHeaderLineParser.parse_line(line)? {
            return Ok(Some(FileHeaderLine::New(line)));
        }
        Ok(None)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OldFileHeaderLineParser;

impl<'a> LineParser<'a> for OldFileHeaderLineParser {
    const NAME: &'static str = "old file header line";

    type Output = Option<Path<'a>>;

    fn parse_line(&self, line: &'a str) -> Result<Option<Self::Output>> {
        let Some(line) = line.strip_prefix("---") else {
            return Ok(None);
        };
        let path = parse_file_header_path(line, "a/")?;
        Ok(Some(path))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NewFileHeaderLineParser;

impl<'a> LineParser<'a> for NewFileHeaderLineParser {
    const NAME: &'static str = "new file header line";

    type Output = Option<Path<'a>>;

    fn parse_line(&self, line: &'a str) -> Result<Option<Self::Output>> {
        let Some(line) = line.strip_prefix("+++") else {
            return Ok(None);
        };
        let path = parse_file_header_path(line, "b/")?;
        Ok(Some(path))
    }
}

fn parse_file_header_path<'a>(line: &'a str, prefix: &str) -> Result<Option<Path<'a>>> {
    ensure!(
        line.starts_with(char::is_whitespace),
        "missing whitespace before file header path"
    );
    let line = line.trim_start();
    if line == "/dev/null" {
        return Ok(None);
    }

    if line.starts_with('"') {
        let (path, rest) = parse_quoted_diff_header_path(line, prefix)?;
        ensure!(rest.trim().is_empty(), "trailing content in file header");
        return Ok(Some(path));
    }

    let Some(path) = line.strip_prefix(prefix) else {
        bail!("missing {prefix:?} in file header path");
    };
    Ok(Some(Path::borrowed(path.trim_end())))
}

#[test]
fn parse_file_header_line() {
    assert_eq!(
        OldFileHeaderLineParser.parse_line_expected("--- a/src/main.rs"),
        Some(Path::borrowed("src/main.rs")),
    );
    assert_eq!(
        NewFileHeaderLineParser.parse_line_expected("+++ b/src/main.rs"),
        Some(Path::borrowed("src/main.rs")),
    );
    assert_eq!(
        OldFileHeaderLineParser.parse_line_expected("--- /dev/null"),
        None,
    );
    assert_eq!(
        NewFileHeaderLineParser.parse_line_expected("+++ /dev/null"),
        None,
    );
    assert_eq!(
        OldFileHeaderLineParser.parse_line_expected(r#"--- "a/foo\tbar""#),
        Some(Path::owned("foo\tbar")),
    );
    assert_eq!(
        NewFileHeaderLineParser.parse_line_expected(r#"+++ "b/foo\\tbar""#),
        Some(Path::owned(r#"foo\tbar"#)),
    );
    assert_eq!(
        FileHeaderLineParser.parse_line_expected("+++ b/src/lib.rs"),
        FileHeaderLine::New(Some(Path::borrowed("src/lib.rs"))),
    );
    let merge_file_header_lines = ["--- a/left.rs", "--- a/right.rs", "+++ b/merged.rs"]
        .into_iter()
        .map(|line| FileHeaderLineParser.parse_line_required(line))
        .collect::<Result<Vec<_>>>()
        .unwrap();
    assert_eq!(
        merge_file_header_lines,
        vec![
            FileHeaderLine::Old(Some(Path::borrowed("left.rs"))),
            FileHeaderLine::Old(Some(Path::borrowed("right.rs"))),
            FileHeaderLine::New(Some(Path::borrowed("merged.rs"))),
        ],
    );
}
