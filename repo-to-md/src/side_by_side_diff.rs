use std::borrow::Cow;
use std::fmt::Write;
use std::ops::Range;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::diff::{
    self, Chunk, ChunkParser, Diff, DiffFile, ExtendedHeaderLine, MultilineParser, Path,
};

const DEV_NULL: &str = "/dev/null";
const CONTEXT_LINES: u32 = 5;
const MIN_TRUNCATION_THRESHOLD: usize = 20;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SideBySideDiff<'a> {
    pub files: Vec<SideBySideFile<'a>>,
}

/// A line from a parsed unified diff hunk.
pub(crate) struct ParsedHunkLine {
    pub content: String,
    pub new_line_number: Option<u32>,
}

/// Parse a unified diff hunk and extract display lines with new-side line numbers.
pub(crate) fn parse_diff_hunk_with_line_numbers(
    diff_hunk: &str,
    line_range: Option<(u32, u32)>,
) -> (Vec<ParsedHunkLine>, bool, bool) {
    let lines = diff_hunk.lines().collect::<Vec<_>>();
    let Ok((chunk, _)) = ChunkParser.parse_lines_required(&lines) else {
        return (Vec::new(), false, false);
    };
    let chunk = SideBySideChunk::from(chunk);

    let mut diff_lines = Vec::new();
    let mut truncated_start = false;
    let mut truncated_end = false;
    let mut current_new_line = u32::try_from(chunk.to_range.start).ok();

    for line in chunk.lines {
        match check_line_in_range(current_new_line, line_range, truncated_start) {
            RangeCheckResult::BeforeRange => {
                truncated_start = true;
            }
            RangeCheckResult::AfterRange => {
                truncated_end = true;
                break;
            }
            RangeCheckResult::Include => {
                let new_line_number = (line.status != LineStatus::Removed)
                    .then_some(current_new_line)
                    .flatten();
                diff_lines.push(ParsedHunkLine {
                    content: match line.status {
                        LineStatus::Removed => line.from.into_owned(),
                        LineStatus::Context | LineStatus::Added => line.to.into_owned(),
                    },
                    new_line_number,
                });
            }
        }

        if line.status != LineStatus::Removed
            && let Some(line_number) = &mut current_new_line
        {
            *line_number += 1;
        }
    }

    (diff_lines, truncated_start, truncated_end)
}

/// Calculate the line range to display based on commented lines.
pub(crate) fn calculate_context_range(
    commented_lines: &[u32],
    total_lines: usize,
) -> Option<(u32, u32)> {
    if commented_lines.is_empty() || total_lines <= MIN_TRUNCATION_THRESHOLD {
        return None;
    }

    let Some(min_line) = commented_lines.iter().min().copied() else {
        unreachable!("commented_lines is non-empty");
    };
    let Some(max_line) = commented_lines.iter().max().copied() else {
        unreachable!("commented_lines is non-empty");
    };

    let start = min_line.saturating_sub(CONTEXT_LINES);
    let end = max_line.saturating_add(CONTEXT_LINES);

    if (end - start) as usize > total_lines * 80 / 100 {
        return None;
    }

    Some((start, end))
}

enum RangeCheckResult {
    BeforeRange,
    Include,
    AfterRange,
}

fn check_line_in_range(
    line_number: Option<u32>,
    line_range: Option<(u32, u32)>,
    already_truncated_start: bool,
) -> RangeCheckResult {
    let Some((start, end)) = line_range else {
        return RangeCheckResult::Include;
    };

    match line_number {
        Some(num) if num < start => RangeCheckResult::BeforeRange,
        Some(num) if num > end => RangeCheckResult::AfterRange,
        Some(_) => RangeCheckResult::Include,
        None if already_truncated_start => RangeCheckResult::Include,
        None => RangeCheckResult::Include,
    }
}

impl SideBySideDiff<'static> {
    pub fn parse(raw_diff: &str) -> Result<Self> {
        let lines = raw_diff.lines().collect::<Vec<_>>();
        Ok(diff::parse(&lines)?.into_static().into())
    }
}

impl<'a> SideBySideDiff<'a> {
    pub fn into_static(self) -> SideBySideDiff<'static> {
        SideBySideDiff {
            files: self
                .files
                .into_iter()
                .map(SideBySideFile::into_static)
                .collect(),
        }
    }

    pub fn find_hunk(&self, path: &str, line: u32) -> Option<&SideBySideChunk<'a>> {
        let file = self
            .files
            .iter()
            .find(|file| file.display_path().as_str() == path)?;

        let line = usize::try_from(line).ok()?;
        for chunk in &file.chunks {
            if line >= chunk.to_range.start && line < chunk.to_range.end {
                return Some(chunk);
            }
        }

        file.chunks.iter().min_by_key(|chunk| {
            let mid = chunk.to_range.start + chunk.to_range.len() / 2;
            mid.abs_diff(line)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SideBySideFile<'a> {
    pub from_path: Path<'a>,
    pub to_path: Path<'a>,
    pub status: FileStatus,
    pub chunks: Vec<SideBySideChunk<'a>>,
}

impl<'a> SideBySideFile<'a> {
    pub fn display_path(&self) -> &Path<'a> {
        if self.to_path.as_str() == DEV_NULL && self.from_path.as_str() != DEV_NULL {
            &self.from_path
        } else {
            &self.to_path
        }
    }

    pub fn previous_path(&self) -> Option<&Path<'a>> {
        let display_path = self.display_path().as_str();
        (self.from_path.as_str() != DEV_NULL && self.from_path.as_str() != display_path)
            .then_some(&self.from_path)
    }

    pub fn into_static(self) -> SideBySideFile<'static> {
        SideBySideFile {
            from_path: self.from_path.into_static(),
            to_path: self.to_path.into_static(),
            status: self.status,
            chunks: self
                .chunks
                .into_iter()
                .map(SideBySideChunk::into_static)
                .collect(),
        }
    }
}

/// Status of a file in the diff
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SideBySideChunk<'a> {
    pub from_range: Range<usize>,
    pub to_range: Range<usize>,
    pub lines: Vec<SideBySideLine<'a>>,
}

impl<'a> SideBySideChunk<'a> {
    pub fn to_unified(&self) -> String {
        let mut result = format!(
            "@@ -{},{} +{},{} @@",
            self.from_range.start,
            self.from_range.len(),
            self.to_range.start,
            self.to_range.len()
        );

        for line in &self.lines {
            match line.status {
                LineStatus::Context => {
                    write!(&mut result, "\n {}", line.to).expect("successful string fmt");
                }
                LineStatus::Added => {
                    write!(&mut result, "\n+{}", line.to).expect("successful string fmt");
                }
                LineStatus::Removed => {
                    write!(&mut result, "\n-{}", line.from).expect("successful string fmt");
                }
            }
        }

        result
    }

    pub fn into_static(self) -> SideBySideChunk<'static> {
        SideBySideChunk {
            from_range: self.from_range,
            to_range: self.to_range,
            lines: self
                .lines
                .into_iter()
                .map(SideBySideLine::into_static)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SideBySideLine<'a> {
    pub status: LineStatus,
    pub from: Cow<'a, str>,
    pub to: Cow<'a, str>,
}

impl<'a> SideBySideLine<'a> {
    pub fn into_static(self) -> SideBySideLine<'static> {
        SideBySideLine {
            status: self.status,
            from: Cow::Owned(self.from.into_owned()),
            to: Cow::Owned(self.to.into_owned()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineStatus {
    Context,
    Added,
    Removed,
}

impl<'a> From<Diff<'a>> for SideBySideDiff<'a> {
    fn from(value: Diff<'a>) -> Self {
        SideBySideDiff {
            files: value.files.into_iter().map(Into::into).collect(),
        }
    }
}

impl<'a> From<DiffFile<'a>> for SideBySideFile<'a> {
    fn from(value: DiffFile<'a>) -> Self {
        let from_path = value
            .header
            .old_files
            .first()
            .cloned()
            .flatten()
            .unwrap_or(Path(Cow::Borrowed(DEV_NULL)));
        let to_path = value
            .header
            .new_file
            .clone()
            .unwrap_or(Path(Cow::Borrowed(DEV_NULL)));
        let status = FileStatus::from(value.header.extended_header.as_slice());
        let chunks = value.chunks.into_iter().map(Into::into).collect();

        SideBySideFile {
            from_path,
            to_path,
            status,
            chunks,
        }
    }
}

impl<'a> From<&[ExtendedHeaderLine<'a>]> for FileStatus {
    fn from(value: &[ExtendedHeaderLine<'a>]) -> Self {
        for line in value {
            match line {
                ExtendedHeaderLine::NewFileMode(_) => return FileStatus::Added,
                ExtendedHeaderLine::DeletedFileMode(_) => return FileStatus::Deleted,
                ExtendedHeaderLine::RenameFrom(_) | ExtendedHeaderLine::RenameTo(_) => {
                    return FileStatus::Renamed;
                }
                _ => {}
            }
        }
        FileStatus::Modified
    }
}

impl<'a> From<Chunk<'a>> for SideBySideChunk<'a> {
    fn from(value: Chunk<'a>) -> Self {
        let from_range = value.from_ranges.head().clone();
        let to_range = value.to_range;
        let lines = value.lines.into_iter().map(Into::into).collect();

        SideBySideChunk {
            from_range,
            to_range,
            lines,
        }
    }
}
