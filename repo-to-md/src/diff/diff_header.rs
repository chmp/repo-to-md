use anyhow::{Result, bail, ensure};

use super::parser::LineParser;
use super::path::{Path, parse_quoted_path};

/// The leading `diff --git` line paths.
///
/// Example: `diff --git a/src/lib.rs b/src/lib.rs` stores `src/lib.rs` as both
/// the left and right paths.
#[derive(PartialEq, Debug, Clone, serde::Serialize)]
pub struct DiffHeader<'a> {
    pub left: Path<'a>,
    pub right: Path<'a>,
}

impl<'a> DiffHeader<'a> {
    pub fn into_static(self) -> DiffHeader<'static> {
        DiffHeader {
            left: self.left.into_static(),
            right: self.right.into_static(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DiffHeaderParser;

impl<'a> LineParser<'a> for DiffHeaderParser {
    const NAME: &'static str = "diff header";

    type Output = DiffHeader<'a>;

    fn parse_line(&self, line: &'a str) -> Result<Option<Self::Output>> {
        let Some(line) = line.strip_prefix("diff") else {
            return Ok(None);
        };
        let line = line.trim_start();
        let line = line.strip_prefix("--git").unwrap_or(line);
        let line = line.trim_start();

        let (left, line) = parse_diff_header_left_path(line)?;
        let (right, rest) = parse_diff_header_path(line.trim_start(), "b/")?;
        ensure!(rest.trim().is_empty(), "trailing content in diff header");

        Ok(Some(DiffHeader { left, right }))
    }
}

fn parse_diff_header_left_path<'a>(line: &'a str) -> Result<(Path<'a>, &'a str)> {
    if line.starts_with('"') {
        return parse_quoted_diff_header_path(line, "a/");
    }

    let Some(line) = line.strip_prefix("a/") else {
        bail!("missing a/ in {line:?}");
    };
    let Some((left_end, right_start)) = find_unquoted_diff_header_split(line) else {
        bail!("missing b/ in {line:?}");
    };

    let left = Path::borrowed(line[..left_end].trim_end());
    let rest = &line[right_start..];
    Ok((left, rest))
}

pub(super) fn parse_diff_header_path<'a>(
    line: &'a str,
    prefix: &str,
) -> Result<(Path<'a>, &'a str)> {
    if line.starts_with('"') {
        return parse_quoted_diff_header_path(line, prefix);
    }

    let Some(path) = line.strip_prefix(prefix) else {
        bail!("missing {prefix:?} in diff header path");
    };
    Ok((Path::borrowed(path.trim_end()), ""))
}

pub(super) fn parse_quoted_diff_header_path<'a>(
    line: &'a str,
    prefix: &str,
) -> Result<(Path<'a>, &'a str)> {
    let Some((path, rest)) = parse_quoted_path(line)? else {
        bail!("missing quoted path");
    };
    let Some(path) = path.strip_prefix(prefix) else {
        bail!("missing {prefix:?} in quoted diff header path");
    };
    Ok((Path::owned(path.to_owned()), rest))
}

fn find_unquoted_diff_header_split(line: &str) -> Option<(usize, usize)> {
    // Unquoted `diff --git a/<left> b/<right>` paths are ambiguous because a path
    // may itself contain ` b/`. This heuristic is not guaranteed to parse every
    // possible path correctly; it only picks the most useful separator we can infer
    // without reading later file headers or extended rename/copy headers.
    find_quoted_right_path_marker(line)
        .or_else(|| find_split_after_file_extension(line))
        .or_else(|| line.find(" b/").map(|split| (split, split + 1)))
}

fn find_quoted_right_path_marker(line: &str) -> Option<(usize, usize)> {
    line.find(" \"b/").map(|split| (split, split + 1))
}

fn find_split_after_file_extension(line: &str) -> Option<(usize, usize)> {
    find_unquoted_right_path_candidates(line)
        .find(|&split| path_ends_with_file_extension(&line[..split]))
        .map(|split| (split, split + 1))
}

fn find_unquoted_right_path_candidates(line: &str) -> impl Iterator<Item = usize> + '_ {
    line.match_indices(" b/").map(|(index, _)| index)
}

fn path_ends_with_file_extension(path: &str) -> bool {
    let Some(file_name) = path.rsplit('/').next() else {
        return false;
    };
    let Some(extension) = file_name.rsplit_once('.').map(|(_, extension)| extension) else {
        return false;
    };
    !extension.is_empty()
        && extension
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[test]
fn parse_diff_header() {
    assert_eq!(
        DiffHeaderParser.parse_line_expected("diff --git a/.gitignore b/.gitignore"),
        DiffHeader {
            left: Path::borrowed(".gitignore"),
            right: Path::borrowed(".gitignore"),
        },
    );
    assert_eq!(
        DiffHeaderParser.parse_line_expected("diff --git a/CLAUDE.md b/AGENTS.md"),
        DiffHeader {
            left: Path::borrowed("CLAUDE.md"),
            right: Path::borrowed("AGENTS.md"),
        },
    );
    assert_eq!(
        DiffHeaderParser
            .parse_line_expected("diff --git a/ with spaces.md b/trailing spaces are ignored.md  "),
        DiffHeader {
            left: Path::borrowed(" with spaces.md"),
            right: Path::borrowed("trailing spaces are ignored.md"),
        },
    );
    assert_eq!(
        DiffHeaderParser.parse_line_expected(
            "diff --git \"a/path\\twith\\ncontrols\" \"b/path\\twith\\ncontrols\""
        ),
        DiffHeader {
            left: Path::owned("path\twith\ncontrols"),
            right: Path::owned("path\twith\ncontrols"),
        },
    );
    assert_eq!(
        DiffHeaderParser.parse_line_expected(r#"diff --git "a/foo\\tbar" "b/foo\\tbar""#),
        DiffHeader {
            left: Path::owned(r#"foo\tbar"#),
            right: Path::owned(r#"foo\tbar"#),
        },
    );
    assert_eq!(
        DiffHeaderParser.parse_line_expected(r#"diff --git "a/foo\tbar" b/plain.md"#),
        DiffHeader {
            left: Path::owned("foo\tbar"),
            right: Path::borrowed("plain.md"),
        },
    );
    assert_eq!(
        DiffHeaderParser.parse_line_expected(r#"diff --git a/plain.md "b/foo\tbar""#),
        DiffHeader {
            left: Path::borrowed("plain.md"),
            right: Path::owned("foo\tbar"),
        },
    );
    assert_eq!(
        DiffHeaderParser.parse_line_expected(r#"diff --git a/foo b/bar "b/foo b/bar""#),
        DiffHeader {
            left: Path::borrowed("foo b/bar"),
            right: Path::owned("foo b/bar"),
        },
    );
    assert_eq!(
        DiffHeaderParser.parse_line_expected(
            "diff --git \"a/quoted\\\\slash\\\"quote\" \"b/quoted\\\\slash\\\"quote\""
        ),
        DiffHeader {
            left: Path::owned("quoted\\slash\"quote"),
            right: Path::owned("quoted\\slash\"quote"),
        },
    );
    assert_eq!(
        DiffHeaderParser.parse_line_expected("diff --git a/License.md b/foo b/License.md"),
        DiffHeader {
            left: Path::borrowed("License.md"),
            right: Path::borrowed("foo b/License.md"),
        },
    );
    assert_eq!(
        DiffHeaderParser.parse_line_expected("diff --git a/foo b/bar b/foo b/bar"),
        DiffHeader {
            left: Path::borrowed("foo"),
            right: Path::borrowed("bar b/foo b/bar"),
        },
    );
}
