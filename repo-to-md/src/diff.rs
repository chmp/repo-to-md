/// Diff parsing utilities for handling unified diff format.
use serde::{Deserialize, Serialize};

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
    pub fn parse(source: &str) -> Self {
        let mut parser = DiffParser::new();
        for line in source.lines() {
            parser.process_line(line);
        }
        parser.finish()
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

        // If not found in exact range, return the closest hunk
        file.hunks.iter().min_by_key(|h| {
            let mid = h.new_start + h.new_count / 2;
            (mid as i64 - line as i64).unsigned_abs()
        })
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
    /// Lines to skip: file markers ('+++', '---') or special markers ('\')
    Skip,
}

/// Classify a diff line to determine how it should be processed.
fn classify_diff_line(line: &str) -> DiffLineKind {
    if line.starts_with("+++") || line.starts_with("---") || line.starts_with('\\') {
        DiffLineKind::Skip
    } else if line.starts_with('+') {
        DiffLineKind::Added
    } else if line.starts_with('-') {
        DiffLineKind::Deleted
    } else {
        DiffLineKind::Context
    }
}

/// Extract the content from a diff line by stripping the prefix character.
///
/// - Added/deleted lines: strip the leading '+' or '-'
/// - Context lines: strip the leading space if present
fn extract_line_content(line: &str, kind: &DiffLineKind) -> String {
    match kind {
        DiffLineKind::Added | DiffLineKind::Deleted => {
            if line.len() > 1 {
                line[1..].to_string()
            } else {
                String::new()
            }
        }
        DiffLineKind::Context => {
            if line.starts_with(' ') && line.len() > 1 {
                line[1..].to_string()
            } else {
                line.to_string()
            }
        }
        DiffLineKind::Skip => String::new(),
    }
}

/// Parser state for processing git diff output line by line.
struct DiffParser {
    files: Vec<FileDiff>,
    current_file: Option<FileDiff>,
    current_hunk: Option<HunkBuilder>,
}

impl DiffParser {
    fn new() -> Self {
        DiffParser {
            files: Vec::new(),
            current_file: None,
            current_hunk: None,
        }
    }

    fn process_line(&mut self, line: &str) {
        if line.starts_with("diff --git") {
            self.start_new_file(line);
        } else if line.starts_with("@@") {
            self.start_new_hunk(line);
        } else if self.current_hunk.is_some() {
            self.process_diff_line(line);
        } else {
            self.process_header_line(line);
        }
    }

    fn start_new_file(&mut self, line: &str) {
        // Save the previous file if any
        if let Some(mut file) = self.current_file.take() {
            if let Some(hunk) = self.current_hunk.take() {
                file.hunks.push(hunk.build());
            }
            self.files.push(file);
        }

        // Parse the file paths from "diff --git a/path b/path"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let old_path = parts[2].strip_prefix("a/").unwrap_or(parts[2]);
            let new_path = parts[3].strip_prefix("b/").unwrap_or(parts[3]);

            self.current_file = Some(FileDiff {
                path: new_path.to_string(),
                old_path: if old_path != new_path {
                    Some(old_path.to_string())
                } else {
                    None
                },
                status: FileStatus::Modified,
                hunks: Vec::new(),
            });
        }
    }

    fn start_new_hunk(&mut self, line: &str) {
        if let Some(ref mut file) = self.current_file {
            // Save previous hunk
            if let Some(hunk) = self.current_hunk.take() {
                file.hunks.push(hunk.build());
            }

            // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@
            if let Some(hunk_info) = parse_hunk_header(line) {
                self.current_hunk = Some(HunkBuilder::new(hunk_info, line.to_string()));
            }
        }
    }

    fn process_header_line(&mut self, line: &str) {
        // Skip --- and +++ header lines
        if line.starts_with("---") || line.starts_with("+++") {
            return;
        }

        // Check for file status markers
        if let Some(ref mut file) = self.current_file {
            if line.starts_with("new file mode") {
                file.status = FileStatus::Added;
            } else if line.starts_with("deleted file mode") {
                file.status = FileStatus::Deleted;
            } else if line.starts_with("rename from") {
                file.status = FileStatus::Renamed;
            }
        }
    }

    fn process_diff_line(&mut self, line: &str) {
        let Some(ref mut hunk) = self.current_hunk else {
            return;
        };

        let kind = classify_diff_line(line);
        let content = extract_line_content(line, &kind);

        match kind {
            DiffLineKind::Added => hunk.add_line(None, Some(&content), LineType::Addition),
            DiffLineKind::Deleted => hunk.add_line(Some(&content), None, LineType::Deletion),
            DiffLineKind::Context => {
                hunk.add_line(Some(&content), Some(&content), LineType::Context)
            }
            DiffLineKind::Skip => {}
        }
    }

    fn finish(mut self) -> SideBySideDiff {
        // Don't forget the last file
        if let Some(mut file) = self.current_file {
            if let Some(hunk) = self.current_hunk {
                file.hunks.push(hunk.build());
            }
            self.files.push(file);
        }

        SideBySideDiff { files: self.files }
    }
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

/// Helper for building a hunk with proper line number tracking
struct HunkBuilder {
    info: HunkInfo,
    header: String,
    rows: Vec<DiffRow>,
    old_line: u32,
    new_line: u32,
    pending_deletions: Vec<LineInfo>,
}

impl HunkBuilder {
    fn new(info: HunkInfo, header: String) -> Self {
        let old_line = info.old_start;
        let new_line = info.new_start;
        HunkBuilder {
            info,
            header,
            rows: Vec::new(),
            old_line,
            new_line,
            pending_deletions: Vec::new(),
        }
    }

    fn add_line(
        &mut self,
        old_content: Option<&str>,
        new_content: Option<&str>,
        line_type: LineType,
    ) {
        match line_type {
            LineType::Context => {
                // Flush any pending deletions first
                self.flush_pending_deletions();

                let old_info = old_content.map(|c| LineInfo {
                    number: self.old_line,
                    content: c.to_string(),
                    line_type: LineType::Context,
                    highlighted_html: None,
                });
                let new_info = new_content.map(|c| LineInfo {
                    number: self.new_line,
                    content: c.to_string(),
                    line_type: LineType::Context,
                    highlighted_html: None,
                });

                self.rows.push(DiffRow {
                    old_line: old_info,
                    new_line: new_info,
                });

                if old_content.is_some() {
                    self.old_line += 1;
                }
                if new_content.is_some() {
                    self.new_line += 1;
                }
            }
            LineType::Deletion => {
                // Queue deletion for potential side-by-side pairing with addition
                if let Some(content) = old_content {
                    self.pending_deletions.push(LineInfo {
                        number: self.old_line,
                        content: content.to_string(),
                        line_type: LineType::Deletion,
                        highlighted_html: None,
                    });
                    self.old_line += 1;
                }
            }
            LineType::Addition => {
                // Try to pair with a pending deletion
                if let Some(deletion) = self.pending_deletions.pop() {
                    // Create a side-by-side row (modified line)
                    let new_info = new_content.map(|c| LineInfo {
                        number: self.new_line,
                        content: c.to_string(),
                        line_type: LineType::Addition,
                        highlighted_html: None,
                    });

                    self.rows.push(DiffRow {
                        old_line: Some(deletion),
                        new_line: new_info,
                    });
                } else {
                    // No deletion to pair with, pure addition
                    let new_info = new_content.map(|c| LineInfo {
                        number: self.new_line,
                        content: c.to_string(),
                        line_type: LineType::Addition,
                        highlighted_html: None,
                    });

                    self.rows.push(DiffRow {
                        old_line: None,
                        new_line: new_info,
                    });
                }

                if new_content.is_some() {
                    self.new_line += 1;
                }
            }
        }
    }

    fn flush_pending_deletions(&mut self) {
        for deletion in self.pending_deletions.drain(..) {
            self.rows.push(DiffRow {
                old_line: Some(deletion),
                new_line: None,
            });
        }
    }

    fn build(mut self) -> DiffHunk {
        // Flush any remaining deletions
        self.flush_pending_deletions();

        DiffHunk {
            old_start: self.info.old_start,
            old_count: self.info.old_count,
            new_start: self.info.new_start,
            new_count: self.info.new_count,
            header: self.header,
            rows: self.rows,
        }
    }
}
