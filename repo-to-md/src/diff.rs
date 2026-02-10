use anyhow::{Result, anyhow, bail};
/// Diff parsing utilities for handling unified diff format.
use serde::{Deserialize, Serialize};

/// Number of context lines to show before/after commented lines in large diffs
const CONTEXT_LINES: u32 = 5;

/// Minimum number of lines in a diff hunk before truncation is considered
const MIN_TRUNCATION_THRESHOLD: usize = 20;

/// A line from a diff hunk with its content and line number.
pub(crate) struct DiffLine {
    pub content: String,
    pub new_line_number: Option<u32>,
}

/// Type of change for a diff line
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineType {
    Context,
    Addition,
    Deletion,
}

/// Information about a single line in the diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineInfo {
    pub number: u32,
    pub content: String,
    pub line_type: LineType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlighted_html: Option<String>,
}

/// A row in the side-by-side diff view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffRow {
    pub old_line: Option<LineInfo>,
    pub new_line: Option<LineInfo>,
}

/// A hunk in a file diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub header: String,
    pub rows: Vec<DiffRow>,
}

/// Diff for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub status: FileStatus,
    pub hunks: Vec<DiffHunk>,
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

/// Complete side-by-side diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideBySideDiff {
    pub files: Vec<FileDiff>,
}

impl SideBySideDiff {
    /// Parse a full git diff output into a side-by-side structure.
    ///
    /// This parses the complete output of `git diff` and converts it to a
    /// structured format suitable for rendering in a side-by-side view.
    pub fn parse(source: &str) -> Result<Self> {
        let (files, _trailing) = try_parse_many(source, parse_file)?;
        Ok(SideBySideDiff { files })
    }
}

impl DiffHunk {
    /// Convert the side-by-side hunk back to unified diff format.
    /// This is needed for the markdown formatter which expects unified diff.
    pub fn to_unified(&self) -> String {
        use std::fmt::Write;

        // NOTE: add newlines to each new line to replicate the action of join
        let mut result = self.header.clone();

        for row in &self.rows {
            match (&row.old_line, &row.new_line) {
                // Context line (same on both sides)
                (Some(old), Some(new))
                    if old.line_type == LineType::Context && new.line_type == LineType::Context =>
                {
                    write!(&mut result, "\n {}", new.content).expect("successful string fmt");
                }
                // Modified line (deletion + addition)
                (Some(old), Some(new))
                    if old.line_type == LineType::Deletion
                        && new.line_type == LineType::Addition =>
                {
                    write!(&mut result, "\n-{}", old.content).expect("successful string fmt");
                    write!(&mut result, "\n+{}", new.content).expect("successful string fmt");
                }
                // Pure deletion
                (Some(old), None) if old.line_type == LineType::Deletion => {
                    write!(&mut result, "\n-{}", old.content).expect("successful string fmt");
                }
                // Pure addition
                (None, Some(new)) if new.line_type == LineType::Addition => {
                    write!(&mut result, "\n+{}", new.content).expect("successful string fmt");
                }
                // Fallback for any other case
                _ => {}
            }
        }

        result
    }

    #[allow(unused)]
    pub fn to_new(&self) -> String {
        let mut result = String::new();
        for line in &self.rows {
            let Some(content) = &line.new_line else {
                continue;
            };
            result.push_str(&content.content);
            result.push('\n');
        }
        result
    }
}

impl SideBySideDiff {
    /// Find the hunk containing a specific line number in a file
    pub fn find_hunk(&self, path: &str, line: u32) -> Option<&DiffHunk> {
        let file = self.files.iter().find(|f| f.path == path)?;

        for hunk in &file.hunks {
            // Check if the line falls within this hunk's range
            let hunk_end = hunk.new_start + hunk.new_count;
            if line >= hunk.new_start && line < hunk_end {
                return Some(hunk);
            }
        }

        None
    }
}

/// Classification of a diff line for parsing purposes.
enum DiffLineKind {
    /// Added line (starts with '+', not '+++')
    Added,
    /// Deleted line (starts with '-', not '---')
    Deleted,
    /// Context line (starts with space or is plain text)
    Context,
}

/// Result of checking whether a line should be included in the output.
enum RangeCheckResult {
    /// Line is within range, include it
    Include,
    /// Line is before the range start
    BeforeRange,
    /// Line is after the range end
    AfterRange,
}

/// Strip the `a/` or `b/` prefix from a diff path, returning the remainder.
///
/// Paths like `/dev/null` or other absolute paths are returned unchanged.
fn parse_diff_path_line<'source>(
    source: &'source str,
    prefix: &str,
) -> Result<(&'source str, &'source str)> {
    let Some((line, rest)) = parse_line(source) else {
        bail!("missing path diff line");
    };
    let Some(line) = line.strip_prefix(prefix) else {
        bail!("missing {prefix}");
    };
    let path = line.trim();
    if let Some(p) = path.strip_prefix("a/") {
        Ok((p, rest))
    } else if let Some(p) = path.strip_prefix("b/") {
        Ok((p, rest))
    } else {
        Ok((path, rest))
    }
}

/// Parse the starting line number from a diff hunk's @@ header.
///
/// Extracts the "new file" starting line number from headers like:
/// `@@ -1,5 +10,7 @@` -> returns `Some(10)`
fn parse_starting_line_number(header: &str) -> Option<u32> {
    parse_hunk_header(header).map(|info| info.new_start)
}

/// Classify a diff line to determine how it should be processed.
fn classify_diff_line(line: &str) -> DiffLineKind {
    if line.starts_with('+') {
        DiffLineKind::Added
    } else if line.starts_with('-') {
        DiffLineKind::Deleted
    } else {
        DiffLineKind::Context
    }
}

/// Check if a line should be included based on the optional range filter.
fn check_line_in_range(
    current_line: Option<u32>,
    line_range: Option<(u32, u32)>,
    truncated_start: bool,
) -> RangeCheckResult {
    let Some((range_start, range_end)) = line_range else {
        return RangeCheckResult::Include;
    };

    match current_line {
        Some(line_num) if line_num < range_start => RangeCheckResult::BeforeRange,
        Some(line_num) if line_num > range_end => RangeCheckResult::AfterRange,
        Some(_) => RangeCheckResult::Include,
        // Deleted lines have no line number - include if we haven't started truncating
        None => {
            if truncated_start {
                RangeCheckResult::BeforeRange
            } else {
                RangeCheckResult::Include
            }
        }
    }
}

/// Extract the content from a diff line by stripping the prefix character.
///
/// - Added/deleted lines: strip the leading '+' or '-'
/// - Context lines: strip the leading space if present
fn extract_line_content(line: &str, kind: &DiffLineKind) -> String {
    let prefix = match kind {
        DiffLineKind::Added => '+',
        DiffLineKind::Deleted => '-',
        DiffLineKind::Context => ' ',
    };

    line.strip_prefix(prefix).unwrap_or(line).to_string()
}

/// Parses a unified diff hunk and extracts lines with their line numbers.
///
/// Processes a diff hunk in unified format (with @@ headers) and tracks line numbers
/// for added and context lines. Optionally filters lines based on a line range.
///
/// # Arguments
///
/// * `diff_hunk` - A unified diff hunk string (starting with "@@")
/// * `line_range` - Optional (start, end) range to filter lines. Lines outside this
///   range are excluded, and truncation flags are set accordingly.
///
/// # Returns
///
/// A tuple of:
/// - Vec of [`DiffLine`] structs with content and line numbers
/// - bool indicating if content was truncated at the start
/// - bool indicating if content was truncated at the end
pub(crate) fn parse_diff_hunk_with_line_numbers(
    diff_hunk: &str,
    line_range: Option<(u32, u32)>,
) -> (Vec<DiffLine>, bool, bool) {
    let mut diff_lines = Vec::new();
    let mut lines = diff_hunk.lines();
    let mut truncated_start = false;
    let mut truncated_end = false;

    // Parse the @@ header to get the starting line number
    let mut current_new_line = lines.next().and_then(parse_starting_line_number);

    for line in lines {
        let kind = classify_diff_line(line);

        // Check range filtering
        match check_line_in_range(current_new_line, line_range, truncated_start) {
            RangeCheckResult::BeforeRange => {
                truncated_start = true;
            }
            RangeCheckResult::AfterRange => {
                truncated_end = true;
                break;
            }
            RangeCheckResult::Include => {
                let content = extract_line_content(line, &kind);
                let new_line_number = if !matches!(kind, DiffLineKind::Deleted) {
                    current_new_line
                } else {
                    None
                };
                diff_lines.push(DiffLine {
                    content,
                    new_line_number,
                });
            }
        }

        // Increment line number for added and context lines (not deleted)
        if !matches!(kind, DiffLineKind::Deleted)
            && let Some(ref mut line_num) = current_new_line
        {
            *line_num += 1;
        }
    }

    (diff_lines, truncated_start, truncated_end)
}

/// Calculate the line range to display based on commented lines.
///
/// For large diffs, determines if truncation is beneficial and calculates
/// the range of lines to show (CONTEXT_LINES before/after commented lines).
///
/// # Arguments
///
/// * `commented_lines` - Line numbers that have comments
/// * `total_lines` - Total number of lines in the diff hunk
///
/// # Returns
///
/// `Some((start, end))` if truncation should be applied, `None` to show full hunk
pub(crate) fn calculate_context_range(
    commented_lines: &[u32],
    total_lines: usize,
) -> Option<(u32, u32)> {
    if commented_lines.is_empty() || total_lines <= MIN_TRUNCATION_THRESHOLD {
        return None; // Show full hunk
    }

    let Some(min_line) = commented_lines.iter().min().copied() else {
        unreachable!("commented_lines is non-empty");
    };
    let Some(max_line) = commented_lines.iter().max().copied() else {
        unreachable!("commented_lines is non-empty");
    };

    let start = min_line.saturating_sub(CONTEXT_LINES);
    let end = max_line.saturating_add(CONTEXT_LINES);

    // If the range covers most of the hunk anyway, don't truncate
    if (end - start) as usize > total_lines * 80 / 100 {
        return None;
    }

    Some((start, end))
}

/// Hunk header info (line numbers and counts)
struct HunkInfo {
    old_start: u32,
    old_count: u32,
    new_start: u32,
    new_count: u32,
}

/// Parse hunk header like "@@ -1,5 +1,7 @@" or "@@ -1 +1,2 @@"
fn parse_hunk_header(header: &str) -> Option<HunkInfo> {
    // Find the range markers
    let rest = header.strip_prefix("@@")?;
    let rest = rest.trim_start();

    // Parse old range: -start,count or -start
    let (old_range, rest) = rest.strip_prefix('-')?.split_once(' ')?;
    let (old_start, old_count) = parse_range(old_range);

    // Parse new range: +start,count or +start
    let rest = rest.trim_start();
    let new_range = rest.strip_prefix('+')?;
    let new_range = new_range.split_whitespace().next()?;
    let (new_start, new_count) = parse_range(new_range);

    Some(HunkInfo {
        old_start,
        old_count,
        new_start,
        new_count,
    })
}

/// Parse "start,count" or just "start" (count defaults to 1)
fn parse_range(range: &str) -> (u32, u32) {
    if let Some((start, count)) = range.split_once(',') {
        (start.parse().unwrap_or(1), count.parse().unwrap_or(1))
    } else {
        (range.parse().unwrap_or(1), 1)
    }
}

/// Parse the next file diff in source.
///
/// Advances past the `diff --git` header line, reads status from header lines,
/// then uses the `---` and `+++` lines to determine the old and new file paths.
///
/// Returns `Ok(None)` when no more `diff --git` headers are found.
fn parse_file(source: &str) -> Result<Option<(FileDiff, &str)>> {
    let Some((header, rest)) = parse_diff_header(source)? else {
        return Ok(None);
    };

    let (hunks, rest) = try_parse_many(rest, parse_hunk)?;
    let file = FileDiff {
        path: header.new_path,
        old_path: header.old_path,
        status: header.status,
        hunks,
    };

    Ok(Some((file, rest)))
}

struct DiffHeader {
    status: FileStatus,
    old_path: Option<String>,
    new_path: String,
}

fn parse_diff_header(source: &str) -> Result<Option<(DiffHeader, &str)>> {
    let Some(rest) = parse_diff_header_start(source) else {
        return Ok(None);
    };
    let (status, rest) = parse_status(rest)?;
    let (old_path, rest) = parse_diff_path_line(rest, "---")?;
    let (new_path, rest) = parse_diff_path_line(rest, "+++")?;

    let header = DiffHeader {
        old_path: if old_path != new_path {
            Some(old_path.to_string())
        } else {
            None
        },
        new_path: new_path.to_string(),
        status,
    };

    Ok(Some((header, rest)))
}

fn parse_diff_header_start(source: &str) -> Option<&str> {
    let (line, rest) = parse_line(source)?;
    if !line.starts_with("diff --git") {
        None
    } else {
        Some(rest)
    }
}

fn parse_status(source: &str) -> Result<(FileStatus, &str)> {
    let mut status = FileStatus::Modified;
    let mut rest = source;
    loop {
        let Some((line, trailing)) = parse_line(rest) else {
            bail!("Incorrect diff header");
        };

        if line.starts_with("---") {
            break;
        }

        rest = trailing;

        if line.starts_with("new file mode") {
            status = FileStatus::Added;
        } else if line.starts_with("deleted file mode") {
            status = FileStatus::Deleted;
        } else if line.starts_with("rename from") {
            status = FileStatus::Renamed;
        }
    }

    Ok((status, rest))
}

/// Parse the next hunk in source
///
/// Returns
///
/// - `Ok(None)` if source does not start with a hunk
/// - `Ok((hunk, trailing))` if the hunk was parsed successfully
fn parse_hunk(source: &str) -> Result<Option<(DiffHunk, &str)>> {
    let Some(((info, header), rest)) = parse_hunk_header_line(source)? else {
        return Ok(None);
    };

    let mut rows = Vec::new();

    let mut rest = rest;
    let mut old_line = info.old_start;
    let mut new_line = info.new_start;
    loop {
        let Some((row, new_rest)) = parse_diff_row(rest, old_line, new_line) else {
            break;
        };
        rest = new_rest;

        if row.old_line.is_some() {
            old_line += 1;
        }
        if row.new_line.is_some() {
            new_line += 1;
        }
        rows.push(row);
    }

    let hunk = DiffHunk {
        old_start: info.old_start,
        old_count: info.old_count,
        new_start: info.new_start,
        new_count: info.new_count,
        header: header.to_string(),
        rows,
    };

    Ok(Some((hunk, rest)))
}

/// Parse the next hunk header line
///
/// Returns
///
/// - `Ok(None)` if source does not start with a hunk header
/// - `Ok(Some((info, header_line), trailing))` if the header could be parsed
fn parse_hunk_header_line(source: &str) -> Result<Option<((HunkInfo, &str), &str)>> {
    let Some((header, rest)) = parse_line(source) else {
        return Ok(None);
    };

    let Some(line) = header.strip_prefix("@@") else {
        return Ok(None);
    };

    let line = line.trim_start();
    let line = parse_required(line, '-')?;
    let (start_range, line) = parse_line_range(line)?;

    let line = line.trim_start();
    let line = parse_required(line, '+')?;
    let (end_range, line) = parse_line_range(line)?;
    let line = line.trim_start();
    let line = parse_required(line, "@@")?;

    // ignore the context
    let _ = line;

    let info = HunkInfo {
        old_start: start_range.0,
        old_count: start_range.1,
        new_start: end_range.0,
        new_count: end_range.1,
    };

    Ok(Some(((info, header), rest)))
}

fn parse_diff_row(source: &str, old_line: u32, new_line: u32) -> Option<(DiffRow, &str)> {
    let (line, rest) = parse_line(source)?;

    let (old, new) = if line.is_empty() {
        (Some(line), Some(line))
    } else if let Some(line) = line.strip_prefix(' ') {
        (Some(line), Some(line))
    } else if let Some(line) = line.strip_prefix('-') {
        (Some(line), None)
    } else if let Some(line) = line.strip_prefix('+') {
        (None, Some(line))
    } else {
        return None;
    };

    let row = match (old, new) {
        (Some(old), Some(new)) => DiffRow {
            old_line: Some(LineInfo {
                number: old_line,
                content: old.to_string(),
                line_type: LineType::Context,
                highlighted_html: None,
            }),
            new_line: Some(LineInfo {
                number: new_line,
                content: new.to_string(),
                line_type: LineType::Context,
                highlighted_html: None,
            }),
        },
        (Some(old), None) => DiffRow {
            old_line: Some(LineInfo {
                number: old_line,
                content: old.to_string(),
                line_type: LineType::Deletion,
                highlighted_html: None,
            }),
            new_line: None,
        },
        (None, Some(new)) => DiffRow {
            old_line: None,
            new_line: Some(LineInfo {
                number: new_line,
                content: new.to_string(),
                line_type: LineType::Addition,
                highlighted_html: None,
            }),
        },
        (None, None) => unreachable!("either or both need to be some"),
    };

    Some((row, rest))
}

fn parse_required(source: &str, prefix: impl Prefix) -> Result<&str> {
    let Some(rest) = prefix.strip_prefix(source) else {
        bail!("missing prefix {}", prefix.display());
    };
    Ok(rest)
}

fn parse_line_range(source: &str) -> Result<((u32, u32), &str)> {
    let (start, source) =
        parse_integer(source)?.ok_or_else(|| anyhow!("mising required start line"))?;
    let (count, source) = if let Some(source) = source.strip_prefix(',') {
        parse_integer(source)?.ok_or_else(|| anyhow!("missing line count after ,"))?
    } else {
        (1, source)
    };

    Ok(((start, count), source))
}

fn parse_integer(source: &str) -> Result<Option<(u32, &str)>> {
    let rest = source.trim_start_matches(|c: char| c.is_ascii_digit());
    let Some(digit) = source.get(..source.len() - rest.len()) else {
        unreachable!("prefix must be a valid utf8 slice");
    };
    if digit.is_empty() {
        return Ok(None);
    }
    Ok(Some((digit.parse::<u32>()?, rest)))
}

fn parse_line(source: &str) -> Option<(&str, &str)> {
    if source.is_empty() {
        None
    } else {
        Some(source.split_once('\n').unwrap_or((source, "")))
    }
}

trait Prefix {
    fn strip_prefix<'source>(&self, source: &'source str) -> Option<&'source str>;
    fn display(&self) -> impl std::fmt::Display;
}

impl Prefix for char {
    fn strip_prefix<'source>(&self, source: &'source str) -> Option<&'source str> {
        source.strip_prefix(*self)
    }

    fn display(&self) -> impl std::fmt::Display {
        std::fmt::from_fn(|f| std::fmt::Display::fmt(self, f))
    }
}

impl Prefix for &[char] {
    fn strip_prefix<'source>(&self, source: &'source str) -> Option<&'source str> {
        source.strip_prefix(*self)
    }

    fn display(&self) -> impl std::fmt::Display {
        std::fmt::from_fn(|f| {
            write!(f, "[")?;
            for (i, c) in self.iter().enumerate() {
                if i == 0 {
                    write!(f, "{c}")?;
                } else {
                    write!(f, ", {c}")?;
                }
            }
            write!(f, "]")
        })
    }
}

impl Prefix for &str {
    fn strip_prefix<'source>(&self, source: &'source str) -> Option<&'source str> {
        source.strip_prefix(*self)
    }

    fn display(&self) -> impl std::fmt::Display {
        std::fmt::from_fn(|f| std::fmt::Display::fmt(self, f))
    }
}

fn try_parse_many<Parser, Item>(source: &str, mut parser: Parser) -> Result<(Vec<Item>, &str)>
where
    Parser: for<'source> FnMut(&'source str) -> Result<Option<(Item, &'source str)>>,
{
    let mut rest = source;
    let mut items = Vec::new();
    while let Some((item, new_rest)) = parser(rest)? {
        items.push(item);
        rest = new_rest;
    }
    Ok((items, rest))
}
