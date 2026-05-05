use std::borrow::Cow;
use std::fmt::Write;
use std::ops::Range;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::diff_v2::{self, Chunk, Diff, DiffFile, ExtendedHeaderLine, Path};

const DEV_NULL: &str = "/dev/null";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SideBySideDiff<'a> {
    pub files: Vec<SideBySideFile<'a>>,
}

impl SideBySideDiff<'static> {
    pub fn parse(raw_diff: &str) -> Result<Self> {
        let lines = raw_diff.lines().collect::<Vec<_>>();
        Ok(diff_v2::parse(&lines)?.into_static().into())
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
