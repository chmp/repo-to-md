use crate::client::{Comment, Issue};
/// Markdown formatting utilities for PR review comments.
use crate::diff::{calculate_context_range, parse_diff_hunk_with_line_numbers};
use crate::language::{detect_language, get_comment_prefix, get_comment_suffix};
use std::collections::HashMap;

/// Formats grouped PR review comments as markdown with inline code blocks.
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
/// * `grouped_comments` - Comments grouped by file path (from [`group_comments_by_file`])
///
/// # Returns
///
/// A formatted markdown string with headings, code blocks, and inline comments.
pub fn format_comments_as_markdown(grouped_comments: HashMap<String, Vec<Comment>>) -> String {
    let mut output = String::new();

    // Add introduction
    output.push_str("# Pull Request Review Comments\n\n");
    output.push_str("Please address the following review comments:\n\n");

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
                    output.push_str(&format!("## `{}` - Line {}\n\n", file_path, min_line));
                } else {
                    output.push_str(&format!(
                        "## `{}` - Lines {}-{}\n\n",
                        file_path, min_line, max_line
                    ));
                }
            }

            // Open code block for this hunk
            output.push_str(&format!("```{}\n", language));

            // Output comments without line numbers at the TOP of the hunk
            for comment in comments_without_line {
                output_comment(&mut output, comment, "", comment_prefix, comment_suffix)
                    .expect("infaillable formatting");
            }

            // Show ellipsis if content was truncated at start
            if truncated_start {
                output.push_str("...\n");
            }

            // Output code with inline comments
            for diff_line in diff_lines {
                // Output the code line
                output.push_str(&diff_line.content);
                output.push('\n');

                // Check if there are comments for this line
                if let Some(line_num) = diff_line.new_line_number
                    && let Some(comments) = comments_by_line.get(&line_num)
                {
                    let indentation = get_indentation(&diff_line.content);
                    for comment in comments {
                        output_comment(
                            &mut output,
                            comment,
                            &indentation,
                            comment_prefix,
                            comment_suffix,
                        )
                        .expect("infaillable formatting");
                    }
                }
            }

            // Show ellipsis if content was truncated at end
            if truncated_end {
                output.push_str("...\n");
            }

            // Close code block for this hunk
            output.push_str("```\n\n");
        }
    }

    output
}

/// Extracts the leading whitespace (indentation) from a line.
fn get_indentation(line: &str) -> String {
    line.chars().take_while(|c| c.is_whitespace()).collect()
}

/// Outputs a comment with language-specific comment syntax.
///
/// Formats a comment with the appropriate prefix/suffix for the language,
/// wrapping it in `<review user="...">` XML tags.
///
/// # Arguments
///
/// * `output` - The string to append the formatted comment to
/// * `comment` - The comment to format
/// * `indentation` - Whitespace to prepend to each line
/// * `prefix` - Language-specific comment prefix (e.g., "//" or "#")
/// * `suffix` - Language-specific comment suffix (e.g., " -->" for HTML)
fn output_comment(
    output: &mut String,
    comment: &Comment,
    indentation: &str,
    prefix: &str,
    suffix: &str,
) -> std::fmt::Result {
    use std::fmt::Write;

    let user_login = &comment.user.login;
    writeln!(
        output,
        "{indentation}{prefix} <review user=\"{user_login}\">{suffix}"
    )?;

    for line in comment.body.lines() {
        if line.is_empty() {
            writeln!(output, "{indentation}{prefix}{suffix}")?;
        } else {
            writeln!(output, "{indentation}{prefix} {line}{suffix}")?;
        }
    }

    writeln!(output, "{indentation}{prefix} </review>{suffix}")
}

/// Formats a GitHub issue as markdown.
///
/// Generates a markdown representation of an issue designed for LLM consumption,
/// including title, state, author, creation date, labels, and description.
///
/// # Arguments
///
/// * `issue` - The issue to format
///
/// # Returns
///
/// A formatted markdown string
pub fn write_issue_as_markdown(
    writer: &mut impl std::io::Write,
    issue: &Issue,
) -> std::io::Result<()> {
    // Title
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
