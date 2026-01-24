use std::collections::HashMap;
use std::io::Write;

use crate::client::{Comment, Issue};
use crate::diff::{calculate_context_range, parse_diff_hunk_with_line_numbers};
use crate::language::{detect_language, get_comment_prefix, get_comment_suffix};

/// Writes grouped PR review comments as markdown with inline code blocks.
///
/// This is the core formatting engine that generates markdown output designed for
/// LLM consumption. For each file:
/// - Groups comments by their diff hunk
/// - Parses diff hunks to extract line numbers
/// - Applies intelligent truncation for large diffs (shows context around commented lines)
/// - Generates markdown code blocks with language-specific syntax highlighting
/// - Embeds comments as inline code comments using `<review user="...">` XML tags
///
/// Comments are embedded at the appropriate line numbers within the code. Comments
/// without line numbers appear at the top of the code block. Large diffs are
/// truncated to show only 5 lines of context before/after commented lines.
///
/// # Arguments
///
/// * `writer` - The writer to output the formatted markdown to
/// * `grouped_comments` - Comments grouped by file path (from [`group_comments_by_file`])
pub fn write_comments_as_markdown(
    writer: &mut impl Write,
    grouped_comments: HashMap<String, Vec<Comment>>,
) -> std::io::Result<()> {
    writeln!(writer, "# Pull Request Review Comments")?;
    writeln!(writer)?;
    writeln!(writer, "Please address the following review comments:")?;
    writeln!(writer)?;

    let mut files: Vec<_> = grouped_comments.keys().collect();
    files.sort();

    for file_path in files {
        let comments = &grouped_comments[file_path];
        let language = detect_language(file_path);
        let comment_prefix = get_comment_prefix(language);
        let comment_suffix = get_comment_suffix(language);

        // Group comments by their diff_hunk to avoid processing same hunk multiple times
        let mut hunks: HashMap<String, Vec<&Comment>> = HashMap::new();
        for comment in comments {
            hunks
                .entry(comment.diff_hunk.clone())
                .or_default()
                .push(comment);
        }

        let mut hunk_list: Vec<_> = hunks.iter().collect();
        hunk_list
            .sort_by_key(|(_, comments)| comments.iter().filter_map(|c| c.line).min().unwrap_or(0));

        for (diff_hunk, hunk_comments) in hunk_list {
            // Separate comments with and without line numbers
            let mut comments_by_line: HashMap<u32, Vec<&Comment>> = HashMap::new();
            let mut comments_without_line: Vec<&Comment> = Vec::new();

            for comment in hunk_comments {
                if let Some(line) = comment.line {
                    comments_by_line.entry(line).or_default().push(comment);
                } else {
                    comments_without_line.push(comment);
                }
            }

            // Calculate context range based on commented lines
            let commented_lines: Vec<u32> = comments_by_line.keys().copied().collect();

            // Parse diff hunk to get total line count first (for threshold check)
            let (all_lines, _, _) = parse_diff_hunk_with_line_numbers(diff_hunk, None);
            let line_range = calculate_context_range(&commented_lines, all_lines.len());

            // Parse diff hunk with line numbers (applying range filter)
            let (diff_lines, truncated_start, truncated_end) =
                parse_diff_hunk_with_line_numbers(diff_hunk, line_range);

            // Calculate line range for the heading
            let line_nums: Vec<u32> = diff_lines
                .iter()
                .filter_map(|dl| dl.new_line_number)
                .collect();

            if !line_nums.is_empty() {
                let min_line = *line_nums.iter().min().unwrap();
                let max_line = *line_nums.iter().max().unwrap();

                if min_line == max_line {
                    writeln!(writer, "## `{file_path}` - Line {min_line}")?;
                } else {
                    writeln!(writer, "## `{file_path}` - Lines {min_line}-{max_line}")?;
                }
                writeln!(writer)?;
            }

            // Open code block for this hunk
            writeln!(writer, "```{language}")?;

            // Output comments without line numbers at the TOP of the hunk
            for comment in comments_without_line {
                write_comment(writer, comment, "", comment_prefix, comment_suffix)?;
            }

            // Show ellipsis if content was truncated at start
            if truncated_start {
                writeln!(writer, "...")?;
            }

            // Output code with inline comments
            for diff_line in diff_lines {
                // Output the code line
                writeln!(writer, "{content}", content = diff_line.content)?;

                // Check if there are comments for this line
                if let Some(line_num) = diff_line.new_line_number
                    && let Some(comments) = comments_by_line.get(&line_num)
                {
                    let indentation = get_indentation(&diff_line.content);
                    for comment in comments {
                        write_comment(
                            writer,
                            comment,
                            &indentation,
                            comment_prefix,
                            comment_suffix,
                        )?;
                    }
                }
            }

            // Show ellipsis if content was truncated at end
            if truncated_end {
                writeln!(writer, "...")?;
            }

            // Close code block for this hunk
            writeln!(writer, "```")?;
            writeln!(writer)?;
        }
    }

    Ok(())
}

/// Extracts the leading whitespace (indentation) from a line.
fn get_indentation(line: &str) -> String {
    line.chars().take_while(|c| c.is_whitespace()).collect()
}

/// Writes a comment with language-specific comment syntax.
///
/// Formats a comment with the appropriate prefix/suffix for the language,
/// wrapping it in `<review user="...">` XML tags.
///
/// # Arguments
///
/// * `writer` - The writer to output the formatted comment to
/// * `comment` - The comment to format
/// * `indentation` - Whitespace to prepend to each line
/// * `prefix` - Language-specific comment prefix (e.g., "//" or "#")
/// * `suffix` - Language-specific comment suffix (e.g., " -->" for HTML)
fn write_comment(
    writer: &mut impl Write,
    comment: &Comment,
    indentation: &str,
    prefix: &str,
    suffix: &str,
) -> std::io::Result<()> {
    let user_login = &comment.user.login;
    writeln!(
        writer,
        "{indentation}{prefix} <review user=\"{user_login}\">{suffix}"
    )?;

    for line in comment.body.lines() {
        if line.is_empty() {
            writeln!(writer, "{indentation}{prefix}{suffix}")?;
        } else {
            writeln!(writer, "{indentation}{prefix} {line}{suffix}")?;
        }
    }

    writeln!(writer, "{indentation}{prefix} </review>{suffix}")
}

/// Writes a GitHub issue as markdown.
///
/// Generates a markdown representation of an issue designed for LLM consumption,
/// including title, state, author, creation date, labels, and description.
///
/// # Arguments
///
/// * `writer` - The writer to output the formatted markdown to
/// * `issue` - The issue to format
pub fn write_issue_as_markdown(writer: &mut impl Write, issue: &Issue) -> std::io::Result<()> {
    writeln!(
        writer,
        "# Issue #{number}: {title}",
        number = issue.number,
        title = issue.title,
    )?;
    writeln!(writer)?;

    writeln!(writer, "- **State:** {state}", state = issue.state)?;

    if let Some(author) = &issue.author {
        writeln!(writer, "- **Author:** @{login}", login = author.login)?;
    }

    writeln!(
        writer,
        "- **Created:** {created}",
        created = issue.created_at
    )?;

    if !issue.labels.is_empty() {
        write!(writer, "- **Labels:** ")?;
        for (idx, label) in issue.labels.iter().enumerate() {
            if idx != 0 {
                write!(writer, ", {name}", name = label.name)?;
            } else {
                write!(writer, "{name}", name = label.name)?;
            }
        }
        writeln!(writer)?;
    }

    writeln!(writer)?;
    writeln!(writer, "## Description")?;
    writeln!(writer)?;
    if let Some(body) = &issue.body
        && !body.is_empty()
    {
        write!(writer, "{body}")?;
        if !body.ends_with('\n') {
            writeln!(writer)?;
        }
    } else {
        writeln!(writer, "*No description provided.*")?;
    }

    Ok(())
}
