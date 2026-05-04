use anyhow::{Result, bail, ensure};

use super::diff_header::parse_quoted_diff_header_path;
use super::parser::LineParser;
use super::path::Path;

#[derive(PartialEq, Debug)]
pub enum FileHeaderLine<'a> {
    // Merge diffs can include multiple old/from file header lines before the
    // new/to file header line.
    Old(OldFileHeaderLine<'a>),
    New(NewFileHeaderLine<'a>),
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FileHeaderLineParser;

impl<'a> LineParser<'a> for FileHeaderLineParser {
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

#[derive(PartialEq, Debug)]
pub struct OldFileHeaderLine<'a> {
    pub path: Option<Path<'a>>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OldFileHeaderLineParser;

impl<'a> LineParser<'a> for OldFileHeaderLineParser {
    type Output = OldFileHeaderLine<'a>;

    fn parse_line(&self, line: &'a str) -> Result<Option<Self::Output>> {
        let Some(line) = line.strip_prefix("---") else {
            return Ok(None);
        };
        let path = parse_file_header_path(line, "a/")?;
        Ok(Some(OldFileHeaderLine { path }))
    }
}

#[derive(PartialEq, Debug)]
pub struct NewFileHeaderLine<'a> {
    pub path: Option<Path<'a>>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NewFileHeaderLineParser;

impl<'a> LineParser<'a> for NewFileHeaderLineParser {
    type Output = NewFileHeaderLine<'a>;

    fn parse_line(&self, line: &'a str) -> Result<Option<Self::Output>> {
        let Some(line) = line.strip_prefix("+++") else {
            return Ok(None);
        };
        let path = parse_file_header_path(line, "b/")?;
        Ok(Some(NewFileHeaderLine { path }))
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
        OldFileHeaderLine {
            path: Some(Path::borrowed("src/main.rs")),
        },
    );
    assert_eq!(
        NewFileHeaderLineParser.parse_line_expected("+++ b/src/main.rs"),
        NewFileHeaderLine {
            path: Some(Path::borrowed("src/main.rs")),
        },
    );
    assert_eq!(
        OldFileHeaderLineParser.parse_line_expected("--- /dev/null"),
        OldFileHeaderLine { path: None },
    );
    assert_eq!(
        NewFileHeaderLineParser.parse_line_expected("+++ /dev/null"),
        NewFileHeaderLine { path: None },
    );
    assert_eq!(
        OldFileHeaderLineParser.parse_line_expected(r#"--- "a/foo\tbar""#),
        OldFileHeaderLine {
            path: Some(Path::owned("foo\tbar")),
        },
    );
    assert_eq!(
        NewFileHeaderLineParser.parse_line_expected(r#"+++ "b/foo\\tbar""#),
        NewFileHeaderLine {
            path: Some(Path::owned(r#"foo\tbar"#)),
        },
    );
    assert_eq!(
        FileHeaderLineParser.parse_line_expected("+++ b/src/lib.rs"),
        FileHeaderLine::New(NewFileHeaderLine {
            path: Some(Path::borrowed("src/lib.rs")),
        }),
    );
    let merge_file_header_lines = ["--- a/left.rs", "--- a/right.rs", "+++ b/merged.rs"]
        .into_iter()
        .map(|line| FileHeaderLineParser.parse_line_required(line))
        .collect::<Result<Vec<_>>>()
        .unwrap();
    assert_eq!(
        merge_file_header_lines,
        vec![
            FileHeaderLine::Old(OldFileHeaderLine {
                path: Some(Path::borrowed("left.rs")),
            }),
            FileHeaderLine::Old(OldFileHeaderLine {
                path: Some(Path::borrowed("right.rs")),
            }),
            FileHeaderLine::New(NewFileHeaderLine {
                path: Some(Path::borrowed("merged.rs")),
            }),
        ],
    );
}
