use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

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
}

/// Applies PR review comments directly to source files as TODO comments.
///
/// Comments are inserted after the target line using language-specific comment
/// syntax, wrapped in `<review user="...">` XML tags.
///
/// # Arguments
///
/// * `comments` - Comments grouped by file path
/// * `repo_root` - The root path of the repository
///
/// # Returns
///
/// An `ApplyResult` containing statistics about the operation.
pub fn apply_comments_to_files(
    comments: HashMap<String, Vec<Comment>>,
    repo_root: &Path,
) -> Result<ApplyResult> {
    let mut result = ApplyResult::default();

    for (file_path, file_comments) in comments {
        let full_path = repo_root.join(&file_path);

        // Skip if file doesn't exist
        if !full_path.exists() {
            for comment in file_comments {
                result.comments_skipped.push(SkippedComment {
                    path: file_path.clone(),
                    reason: "File not found".to_string(),
                });
                eprintln!("Skipping comment: file not found: {}", file_path);
                // Only report once for missing file
                if comment.line.is_none() {
                    continue;
                }
            }
            continue;
        }

        // Read the file
        let content = fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read file: {}", full_path.display()))?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        // Filter comments with line numbers and sort descending
        let mut applicable_comments: Vec<&Comment> = file_comments
            .iter()
            .filter(|c| {
                if c.line.is_none() {
                    result.comments_skipped.push(SkippedComment {
                        path: file_path.clone(),
                        reason: "Comment has no line number".to_string(),
                    });
                    eprintln!(
                        "Skipping comment without line number in {}: {}",
                        file_path,
                        truncate_body(&c.body, 50)
                    );
                    false
                } else {
                    true
                }
            })
            .collect();

        if applicable_comments.is_empty() {
            continue;
        }

        // Sort by line number descending (insert from bottom to top)
        applicable_comments.sort_by(|a, b| b.line.unwrap().cmp(&a.line.unwrap()));

        let language = detect_language(&file_path);
        let prefix = get_comment_prefix(language);
        let suffix = get_comment_suffix(language);

        let mut comments_applied_for_file = 0;

        for comment in applicable_comments {
            let line_num = comment.line.unwrap() as usize;

            // Check bounds
            if line_num == 0 || line_num > lines.len() {
                result.comments_skipped.push(SkippedComment {
                    path: file_path.clone(),
                    reason: format!(
                        "Line {} out of bounds (file has {} lines)",
                        line_num,
                        lines.len()
                    ),
                });
                eprintln!(
                    "Skipping comment: line {} out of bounds in {} (file has {} lines)",
                    line_num,
                    file_path,
                    lines.len()
                );
                continue;
            }

            // Get indentation from target line
            let target_line = &lines[line_num - 1];
            let indentation = get_indentation(target_line);

            // Format the comment
            let formatted = format_comment_for_insertion(comment, &indentation, prefix, suffix);

            // Insert after the target line
            lines.insert(line_num, formatted);
            comments_applied_for_file += 1;
        }

        if comments_applied_for_file > 0 {
            // Write the modified content back
            let mut file = fs::File::create(&full_path)
                .with_context(|| format!("Failed to write file: {}", full_path.display()))?;
            for (i, line) in lines.iter().enumerate() {
                if i > 0 {
                    writeln!(file)?;
                }
                write!(file, "{}", line)?;
            }
            // Ensure file ends with newline
            writeln!(file)?;

            result.files_modified += 1;
            result.comments_applied += comments_applied_for_file;
        }
    }

    Ok(result)
}

/// Formats a comment for insertion into a source file.
///
/// The comment is formatted as:
/// ```text
/// {indent}{prefix} TODO: <review user="{user}">{suffix}
/// {indent}{prefix} {comment_line_1}{suffix}
/// {indent}{prefix} </review>{suffix}
/// ```
fn format_comment_for_insertion(
    comment: &Comment,
    indentation: &str,
    prefix: &str,
    suffix: &str,
) -> String {
    let mut lines = Vec::new();

    // Opening tag with TODO
    lines.push(format!(
        "{}{} TODO: <review user=\"{}\">{}",
        indentation, prefix, comment.user.login, suffix
    ));

    // Comment body lines
    for line in comment.body.lines() {
        if line.is_empty() {
            lines.push(format!("{}{}{}", indentation, prefix, suffix));
        } else {
            lines.push(format!("{}{} {}{}", indentation, prefix, line, suffix));
        }
    }

    // Closing tag
    lines.push(format!("{}{} </review>{}", indentation, prefix, suffix));

    lines.join("\n")
}

/// Extracts the leading whitespace (indentation) from a line.
fn get_indentation(line: &str) -> String {
    line.chars().take_while(|c| c.is_whitespace()).collect()
}

/// Truncates a string to a maximum length, adding "..." if truncated.
fn truncate_body(s: &str, max_len: usize) -> String {
    let first_line = s.lines().next().unwrap_or(s);
    if first_line.len() > max_len {
        format!("{}...", &first_line[..max_len])
    } else {
        first_line.to_string()
    }
}

/// Prints the result of applying comments to files.
pub fn print_apply_result(result: &ApplyResult, writer: &mut impl Write) -> Result<()> {
    writeln!(
        writer,
        "Applied {} comment(s) to {} file(s)",
        result.comments_applied, result.files_modified
    )?;

    if !result.comments_skipped.is_empty() {
        writeln!(
            writer,
            "Skipped {} comment(s)",
            result.comments_skipped.len()
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::User;
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

        assert!(result.contains("    // TODO: <review user=\"reviewer\">"));
        assert!(result.contains("    // Fix this bug"));
        assert!(result.contains("    // </review>"));
    }

    #[test]
    fn test_format_comment_for_insertion_python() {
        let comment = create_test_comment("test.py", Some(10), "Add docstring", "reviewer");
        let result = format_comment_for_insertion(&comment, "  ", "#", "");

        assert!(result.contains("  # TODO: <review user=\"reviewer\">"));
        assert!(result.contains("  # Add docstring"));
        assert!(result.contains("  # </review>"));
    }

    #[test]
    fn test_format_comment_multiline() {
        let comment =
            create_test_comment("test.rs", Some(10), "Line 1\nLine 2\nLine 3", "reviewer");
        let result = format_comment_for_insertion(&comment, "", "//", "");

        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 5);
        assert!(lines[0].contains("TODO: <review"));
        assert!(lines[1].contains("Line 1"));
        assert!(lines[2].contains("Line 2"));
        assert!(lines[3].contains("Line 3"));
        assert!(lines[4].contains("</review>"));
    }

    #[test]
    fn test_get_indentation() {
        assert_eq!(get_indentation("    code"), "    ");
        assert_eq!(get_indentation("\t\tcode"), "\t\t");
        assert_eq!(get_indentation("code"), "");
        assert_eq!(get_indentation("  \t  code"), "  \t  ");
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
        assert!(content.contains("    // TODO: <review"));
        assert!(content.contains("    // Consider renaming"));
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
        assert!(content.contains("Comment on line 1"));
        assert!(content.contains("Comment on line 3"));

        // Verify order is preserved (line 1 content comes before line 3 content)
        let pos1 = content.find("line 1").unwrap();
        let pos3 = content.find("line 3").unwrap();
        assert!(pos1 < pos3);
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
        assert!(!result.comments_skipped.is_empty());
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
        assert!(result.comments_skipped[0].reason.contains("no line number"));
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
}
