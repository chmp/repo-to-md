use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

use anyhow::{bail, Context, Result};

use crate::client::Comment;
use crate::language::{detect_language, get_comment_prefix, get_comment_suffix};

/// Result of applying comments to source files.
#[derive(Debug, Default)]
pub struct ApplyResult {
    /// Number of files that were modified.
    pub files_modified: usize,
    /// Number of comments that were successfully applied.
    pub comments_applied: usize,
    /// Comments that were skipped (file not found, no line number, etc.).
    pub comments_skipped: Vec<SkippedComment>,
}

/// A comment that was skipped during apply.
#[derive(Debug)]
pub struct SkippedComment {
    /// The file path the comment was for.
    pub path: String,
    /// The reason the comment was skipped.
    pub reason: String,
    /// Preview of the comment body.
    pub body_preview: String,
}

/// A wrapper for truncating strings in Display without allocation.
struct Truncated<'a> {
    s: &'a str,
    max_len: usize,
}

impl<'a> Truncated<'a> {
    fn new(s: &'a str, max_len: usize) -> Self {
        Self { s, max_len }
    }
}

impl fmt::Display for Truncated<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let first_line = self.s.lines().next().unwrap_or(self.s);
        if first_line.len() > self.max_len {
            write!(f, "{}...", &first_line[..self.max_len])
        } else {
            write!(f, "{}", first_line)
        }
    }
}

/// Applies PR review comments directly to source files as TODO comments.
///
/// Comments are inserted after the target line using language-specific comment
/// syntax, wrapped in `<review user="...">` XML tags.
pub fn apply_comments_to_files(
    comments: HashMap<String, Vec<Comment>>,
    repo_root: &Path,
) -> Result<ApplyResult> {
    let mut result = ApplyResult::default();

    for (file_path, file_comments) in comments {
        apply_comments_to_single_file(&file_path, file_comments, repo_root, &mut result)?;
    }

    Ok(result)
}

fn apply_comments_to_single_file(
    file_path: &str,
    file_comments: Vec<Comment>,
    repo_root: &Path,
    result: &mut ApplyResult,
) -> Result<()> {
    let full_path = repo_root.join(file_path);

    if !full_path.exists() {
        skip_comments_for_missing_file(file_path, &file_comments, result);
        return Ok(());
    }

    eprintln!("Applying comments to {}", file_path);

    let original_mtime = get_modification_time(&full_path)?;
    let content = fs::read_to_string(&full_path)
        .with_context(|| format!("Failed to read file: {}", full_path.display()))?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    let applicable_comments = filter_applicable_comments(file_path, &file_comments, result);

    if applicable_comments.is_empty() {
        return Ok(());
    }

    let language = detect_language(file_path);
    let prefix = get_comment_prefix(language);
    let suffix = get_comment_suffix(language);

    let comments_applied_for_file = insert_comments_into_lines(
        &mut lines,
        &applicable_comments,
        file_path,
        prefix,
        suffix,
        result,
    );

    if comments_applied_for_file > 0 {
        if original_mtime != get_modification_time(&full_path)? {
            bail!(
                "File {} was modified while processing. Aborting to prevent data loss.",
                full_path.display()
            );
        }

        write_file(&full_path, &lines)?;
        result.files_modified += 1;
        result.comments_applied += comments_applied_for_file;
    }

    Ok(())
}

fn skip_comments_for_missing_file(
    file_path: &str,
    file_comments: &[Comment],
    result: &mut ApplyResult,
) {
    eprintln!("Skipping file (not found): {}", file_path);
    for comment in file_comments {
        result.comments_skipped.push(SkippedComment {
            path: file_path.to_string(),
            reason: "File not found".to_string(),
            body_preview: Truncated::new(&comment.body, 50).to_string(),
        });
    }
}

fn filter_applicable_comments<'a>(
    file_path: &str,
    file_comments: &'a [Comment],
    result: &mut ApplyResult,
) -> Vec<(u32, &'a Comment)> {
    let mut applicable: Vec<(u32, &Comment)> = Vec::new();

    for comment in file_comments {
        match comment.line {
            Some(line) => applicable.push((line, comment)),
            None => {
                eprintln!(
                    "Skipping comment without line number in {}: {}",
                    file_path,
                    Truncated::new(&comment.body, 50)
                );
                result.comments_skipped.push(SkippedComment {
                    path: file_path.to_string(),
                    reason: "Comment has no line number".to_string(),
                    body_preview: Truncated::new(&comment.body, 50).to_string(),
                });
            }
        }
    }

    // Sort by line number descending (insert from bottom to top)
    applicable.sort_by(|a, b| b.0.cmp(&a.0));
    applicable
}

fn insert_comments_into_lines(
    lines: &mut Vec<String>,
    applicable_comments: &[(u32, &Comment)],
    file_path: &str,
    prefix: &str,
    suffix: &str,
    result: &mut ApplyResult,
) -> usize {
    let mut comments_applied = 0;

    for &(line_num, comment) in applicable_comments {
        let line_num_usize = line_num as usize;

        if line_num_usize == 0 || line_num_usize > lines.len() {
            eprintln!(
                "Skipping comment: line {} out of bounds in {} (file has {} lines)",
                line_num,
                file_path,
                lines.len()
            );
            result.comments_skipped.push(SkippedComment {
                path: file_path.to_string(),
                reason: format!(
                    "Line {} out of bounds (file has {} lines)",
                    line_num,
                    lines.len()
                ),
                body_preview: Truncated::new(&comment.body, 50).to_string(),
            });
            continue;
        }

        let target_line = &lines[line_num_usize - 1];
        let indentation = get_indentation(target_line);
        let formatted = format_comment_for_insertion(comment, &indentation, prefix, suffix);

        lines.insert(line_num_usize, formatted);
        comments_applied += 1;
    }

    comments_applied
}

fn get_modification_time(path: &Path) -> Result<SystemTime> {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .with_context(|| format!("Failed to get modification time for {}", path.display()))
}

fn write_file(path: &Path, lines: &[String]) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to write file: {}", path.display()))?;

    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            writeln!(file)?;
        }
        write!(file, "{}", line)?;
    }
    writeln!(file)?;

    Ok(())
}

/// Formats a comment for insertion into a source file.
fn format_comment_for_insertion(
    comment: &Comment,
    indentation: &str,
    prefix: &str,
    suffix: &str,
) -> String {
    let mut output = Vec::new();

    output.push(format!(
        "{}{} <review user=\"{}\">{}",
        indentation, prefix, comment.user.login, suffix
    ));

    for line in comment.body.lines() {
        if line.is_empty() {
            output.push(format!("{}{}{}", indentation, prefix, suffix));
        } else {
            output.push(format!("{}{} {}{}", indentation, prefix, line, suffix));
        }
    }

    output.push(format!("{}{} </review>{}", indentation, prefix, suffix));

    output.join("\n")
}

fn get_indentation(line: &str) -> String {
    line.chars().take_while(|c| c.is_whitespace()).collect()
}

/// Prints the result of applying comments to files.
pub fn print_apply_result(result: &ApplyResult, writer: &mut impl Write) -> Result<()> {
    writeln!(
        writer,
        "Applied {} comment(s) to {} file(s)",
        result.comments_applied, result.files_modified
    )?;

    if !result.comments_skipped.is_empty() {
        writeln!(writer)?;
        writeln!(
            writer,
            "Skipped {} comment(s):",
            result.comments_skipped.len()
        )?;
        for skipped in &result.comments_skipped {
            writeln!(
                writer,
                "  - {}: {} ({})",
                skipped.path, skipped.reason, skipped.body_preview
            )?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::User;
    use insta::assert_snapshot;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_comment(path: &str, line: Option<u32>, body: &str, user: &str) -> Comment {
        Comment {
            path: path.to_string(),
            line,
            body: body.to_string(),
            diff_hunk: String::new(),
            user: User {
                login: user.to_string(),
            },
        }
    }

    #[test]
    fn test_format_comment_for_insertion_rust() {
        let comment = create_test_comment("test.rs", Some(10), "Fix this bug", "reviewer");
        let result = format_comment_for_insertion(&comment, "    ", "//", "");
        assert_snapshot!(result);
    }

    #[test]
    fn test_format_comment_for_insertion_python() {
        let comment = create_test_comment("test.py", Some(10), "Add docstring", "reviewer");
        let result = format_comment_for_insertion(&comment, "  ", "#", "");
        assert_snapshot!(result);
    }

    #[test]
    fn test_format_comment_multiline() {
        let comment =
            create_test_comment("test.rs", Some(10), "Line 1\nLine 2\nLine 3", "reviewer");
        let result = format_comment_for_insertion(&comment, "", "//", "");
        assert_snapshot!(result);
    }

    #[test]
    fn test_format_comment_html() {
        let comment = create_test_comment("test.html", Some(5), "Add alt text", "reviewer");
        let result = format_comment_for_insertion(&comment, "  ", "<!--", " -->");
        assert_snapshot!(result);
    }

    #[test]
    fn test_get_indentation() {
        assert_eq!(get_indentation("    code"), "    ");
        assert_eq!(get_indentation("\t\tcode"), "\t\t");
        assert_eq!(get_indentation("code"), "");
        assert_eq!(get_indentation("  \t  code"), "  \t  ");
    }

    #[test]
    fn test_truncated_display_short() {
        let result = format!("{}", Truncated::new("short text", 50));
        assert_eq!(result, "short text");
    }

    #[test]
    fn test_truncated_display_long() {
        let result = format!(
            "{}",
            Truncated::new("this is a very long text that exceeds the limit", 20)
        );
        assert_eq!(result, "this is a very long ...");
    }

    #[test]
    fn test_truncated_display_multiline() {
        let result = format!("{}", Truncated::new("first line\nsecond line", 50));
        assert_eq!(result, "first line");
    }

    #[test]
    fn test_apply_preserves_indentation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        fs::write(&file_path, "fn main() {\n    let x = 1;\n}\n").unwrap();

        let comments = HashMap::from([(
            "test.rs".to_string(),
            vec![create_test_comment(
                "test.rs",
                Some(2),
                "Consider renaming",
                "reviewer",
            )],
        )]);

        let result = apply_comments_to_files(comments, temp_dir.path()).unwrap();

        assert_eq!(result.files_modified, 1);
        assert_eq!(result.comments_applied, 1);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_snapshot!(content);
    }

    #[test]
    fn test_apply_multiple_comments_same_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        fs::write(&file_path, "line 1\nline 2\nline 3\nline 4\n").unwrap();

        let comments = HashMap::from([(
            "test.rs".to_string(),
            vec![
                create_test_comment("test.rs", Some(1), "Comment on line 1", "alice"),
                create_test_comment("test.rs", Some(3), "Comment on line 3", "bob"),
            ],
        )]);

        let result = apply_comments_to_files(comments, temp_dir.path()).unwrap();

        assert_eq!(result.files_modified, 1);
        assert_eq!(result.comments_applied, 2);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_snapshot!(content);
    }

    #[test]
    fn test_apply_skips_missing_file() {
        let temp_dir = TempDir::new().unwrap();

        let comments = HashMap::from([(
            "nonexistent.rs".to_string(),
            vec![create_test_comment(
                "nonexistent.rs",
                Some(1),
                "Comment",
                "reviewer",
            )],
        )]);

        let result = apply_comments_to_files(comments, temp_dir.path()).unwrap();

        assert_eq!(result.files_modified, 0);
        assert_eq!(result.comments_applied, 0);
        assert_eq!(result.comments_skipped.len(), 1);
        assert_eq!(result.comments_skipped[0].reason, "File not found");
    }

    #[test]
    fn test_apply_skips_comment_without_line() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        fs::write(&file_path, "fn main() {}\n").unwrap();

        let comments = HashMap::from([(
            "test.rs".to_string(),
            vec![create_test_comment(
                "test.rs",
                None,
                "General comment",
                "reviewer",
            )],
        )]);

        let result = apply_comments_to_files(comments, temp_dir.path()).unwrap();

        assert_eq!(result.files_modified, 0);
        assert_eq!(result.comments_applied, 0);
        assert_eq!(result.comments_skipped.len(), 1);
        assert_eq!(
            result.comments_skipped[0].reason,
            "Comment has no line number"
        );
    }

    #[test]
    fn test_apply_skips_line_out_of_bounds() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        fs::write(&file_path, "line 1\nline 2\n").unwrap();

        let comments = HashMap::from([(
            "test.rs".to_string(),
            vec![create_test_comment(
                "test.rs",
                Some(100),
                "Comment",
                "reviewer",
            )],
        )]);

        let result = apply_comments_to_files(comments, temp_dir.path()).unwrap();

        assert_eq!(result.files_modified, 0);
        assert_eq!(result.comments_applied, 0);
        assert_eq!(result.comments_skipped.len(), 1);
        assert!(result.comments_skipped[0].reason.contains("out of bounds"));
    }

    #[test]
    fn test_print_apply_result_with_skipped() {
        let result = ApplyResult {
            files_modified: 2,
            comments_applied: 5,
            comments_skipped: vec![
                SkippedComment {
                    path: "missing.rs".to_string(),
                    reason: "File not found".to_string(),
                    body_preview: "Some comment".to_string(),
                },
                SkippedComment {
                    path: "test.rs".to_string(),
                    reason: "Line 100 out of bounds".to_string(),
                    body_preview: "Another comment".to_string(),
                },
            ],
        };

        let mut output = Vec::new();
        print_apply_result(&result, &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        assert_snapshot!(output_str);
    }

    #[test]
    fn test_print_apply_result_no_skipped() {
        let result = ApplyResult {
            files_modified: 1,
            comments_applied: 3,
            comments_skipped: vec![],
        };

        let mut output = Vec::new();
        print_apply_result(&result, &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        assert_snapshot!(output_str);
    }
}
