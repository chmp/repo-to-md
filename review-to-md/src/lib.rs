use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

/// Number of context lines to show before/after commented lines in large diffs
const CONTEXT_LINES: u32 = 5;

/// Minimum number of lines in a diff hunk before truncation is considered
const MIN_TRUNCATION_THRESHOLD: usize = 20;

#[derive(Debug, Deserialize)]
pub struct Comment {
    pub path: String,
    pub line: Option<u32>,
    pub body: String,
    pub diff_hunk: String,
    pub user: User,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub login: String,
}

pub fn get_repo_info() -> Result<(String, String)> {
    let output = Command::new("git")
        .args(&["remote", "get-url", "origin"])
        .output()
        .context("Failed to execute git command")?;

    if !output.status.success() {
        anyhow::bail!("Git remote not configured");
    }

    let url = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in git remote URL")?
        .trim()
        .to_string();

    parse_github_url(&url)
}

pub fn parse_github_url(url: &str) -> Result<(String, String)> {
    if let Some(ssh_match) = url.strip_prefix("git@github.com:") {
        let parts: Vec<&str> = ssh_match.trim_end_matches(".git").split('/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    if let Some(https_match) = url.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = https_match.trim_end_matches(".git").split('/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    anyhow::bail!("Could not parse GitHub URL: {}", url)
}

pub fn fetch_pr_comments(owner: &str, repo: &str, pr_id: u32) -> Result<Vec<Comment>> {
    let api_path = format!("/repos/{}/{}/pulls/{}/comments", owner, repo, pr_id);

    let output = Command::new("gh")
        .args(&[
            "api",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "X-GitHub-Api-Version: 2022-11-28",
            &api_path,
        ])
        .output()
        .context("Failed to execute gh command. Is the gh CLI installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh command failed: {}", stderr);
    }

    let json = String::from_utf8(output.stdout).context("Invalid UTF-8 in gh output")?;
    parse_comments_json(&json)
}

pub fn parse_comments_json(json: &str) -> Result<Vec<Comment>> {
    let comments: Vec<Comment> = serde_json::from_str(json)
        .context("Failed to parse JSON response from GitHub API")?;
    Ok(comments)
}

pub fn read_comments_from_file(file_path: &str) -> Result<Vec<Comment>> {
    let json = std::fs::read_to_string(file_path)
        .context(format!("Failed to read JSON file: {}", file_path))?;
    parse_comments_json(&json)
}

pub fn group_comments_by_file(comments: Vec<Comment>) -> HashMap<String, Vec<Comment>> {
    let mut grouped: HashMap<String, Vec<Comment>> = HashMap::new();
    for comment in comments {
        grouped.entry(comment.path.clone()).or_default().push(comment);
    }
    grouped
}

pub fn detect_language(path: &str) -> &str {
    if let Some(ext) = path.rsplit('.').next() {
        match ext {
            "rs" => "rust",
            "py" => "python",
            "md" => "markdown",
            "toml" => "toml",
            "js" => "javascript",
            "ts" => "typescript",
            "jsx" => "javascript",
            "tsx" => "typescript",
            "c" | "h" => "c",
            "cpp" | "cc" | "hpp" => "cpp",
            "java" => "java",
            "go" => "go",
            "rb" => "ruby",
            "sh" | "bash" => "bash",
            "yaml" | "yml" => "yaml",
            "json" => "json",
            "html" => "html",
            "css" => "css",
            _ => "",
        }
    } else {
        ""
    }
}

pub fn get_comment_prefix(language: &str) -> &str {
    match language {
        "python" | "bash" | "ruby" | "yaml" | "toml" => "#",
        "html" | "markdown" => "<!--",
        _ => "//",
    }
}

pub fn get_comment_suffix(language: &str) -> &str {
    match language {
        "html" | "markdown" => " -->",
        _ => "",
    }
}

struct DiffLine {
    content: String,
    new_line_number: Option<u32>,
}

fn parse_diff_hunk_with_line_numbers(
    diff_hunk: &str,
    line_range: Option<(u32, u32)>,
) -> (Vec<DiffLine>, bool, bool) {
    let mut diff_lines = Vec::new();
    let mut lines = diff_hunk.lines();
    let mut truncated_start = false;
    let mut truncated_end = false;

    // Parse the @@ header to get the starting line number
    let mut current_new_line: Option<u32> = None;

    if let Some(first_line) = lines.next() {
        if first_line.starts_with("@@") {
            // Extract starting line number from header like "@@ -55,6 +59,8 @@"
            // We need the number after the '+' sign
            if let Some(plus_pos) = first_line.find('+') {
                let after_plus = &first_line[plus_pos + 1..];
                if let Some(comma_or_space) = after_plus.find(|c: char| c == ',' || c == ' ') {
                    if let Ok(line_num) = after_plus[..comma_or_space].parse::<u32>() {
                        current_new_line = Some(line_num);
                    }
                }
            }
        }
    }

    // Process each line after the @@ header
    for line in lines {
        // Skip file markers and special lines
        if line.starts_with("+++") || line.starts_with("---") || line.starts_with('\\') {
            continue;
        }

        // Check if we should include this line based on line_range
        let should_include = if let Some((range_start, range_end)) = line_range {
            if let Some(line_num) = current_new_line {
                if line_num < range_start {
                    truncated_start = true;
                    false
                } else if line_num > range_end {
                    truncated_end = true;
                    break; // Stop processing, we're past the range
                } else {
                    true
                }
            } else {
                // Deleted lines (no line number) - include if we're within range
                !truncated_start
            }
        } else {
            true // No range filter, include everything
        };

        if let Some('+') = line.chars().next() {
            // Added line
            if !line.starts_with("+++") {
                let content = if line.len() > 1 { line[1..].to_string() } else { String::new() };
                if should_include {
                    diff_lines.push(DiffLine {
                        content,
                        new_line_number: current_new_line,
                    });
                }
                if let Some(ref mut line_num) = current_new_line {
                    *line_num += 1;
                }
            }
        } else if let Some('-') = line.chars().next() {
            // Deleted line - include in output but no line number
            if !line.starts_with("---") {
                let content = if line.len() > 1 { line[1..].to_string() } else { String::new() };
                if should_include {
                    diff_lines.push(DiffLine {
                        content,
                        new_line_number: None,
                    });
                }
                // Don't increment line number for deleted lines
            }
        } else {
            // Context line (starts with space or nothing)
            let content = if line.starts_with(' ') && line.len() > 1 {
                line[1..].to_string()
            } else {
                line.to_string()
            };
            if should_include {
                diff_lines.push(DiffLine {
                    content,
                    new_line_number: current_new_line,
                });
            }
            if let Some(ref mut line_num) = current_new_line {
                *line_num += 1;
            }
        }
    }

    (diff_lines, truncated_start, truncated_end)
}

fn output_comment(output: &mut String, comment: &Comment, prefix: &str, suffix: &str) {
    output.push_str(&format!("{} <review user=\"{}\">\n", prefix, comment.user.login));

    // Handle multi-line comments: each line gets the prefix
    for line in comment.body.lines() {
        if line.is_empty() {
            output.push_str(&format!("{}\n", prefix));
        } else {
            output.push_str(&format!("{} {}\n", prefix, line));
        }
    }

    output.push_str(&format!("{} </review>{}\n", prefix, suffix));
}

/// Calculate the line range to display based on commented lines
fn calculate_context_range(
    commented_lines: &[u32],
    total_lines: usize,
) -> Option<(u32, u32)> {
    if commented_lines.is_empty() || total_lines <= MIN_TRUNCATION_THRESHOLD {
        return None; // Show full hunk
    }

    let min_line = *commented_lines.iter().min().unwrap();
    let max_line = *commented_lines.iter().max().unwrap();

    let start = min_line.saturating_sub(CONTEXT_LINES);
    let end = max_line.saturating_add(CONTEXT_LINES);

    // If the range covers most of the hunk anyway, don't truncate
    if (end - start) as usize > total_lines * 80 / 100 {
        return None;
    }

    Some((start, end))
}

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
            hunks.entry(comment.diff_hunk.clone()).or_default().push(comment);
        }

        let mut hunk_list: Vec<_> = hunks.iter().collect();
        hunk_list.sort_by_key(|(_, comments)| {
            comments.iter().filter_map(|c| c.line).min().unwrap_or(0)
        });

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
                    output.push_str(&format!("## `{}` - Lines {}-{}\n\n", file_path, min_line, max_line));
                }
            }

            // Open code block for this hunk
            output.push_str(&format!("```{}\n", language));

            // Output comments without line numbers at the TOP of the hunk
            for comment in comments_without_line {
                output_comment(&mut output, comment, comment_prefix, comment_suffix);
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
                if let Some(line_num) = diff_line.new_line_number {
                    if let Some(comments) = comments_by_line.get(&line_num) {
                        // Output ALL comments for this line
                        for comment in comments {
                            output_comment(&mut output, comment, comment_prefix, comment_suffix);
                        }
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

pub fn extract_code_from_diff_hunk(diff_hunk: &str) -> Vec<String> {
    let mut code_lines = Vec::new();

    for line in diff_hunk.lines() {
        if line.starts_with("@@") {
            continue;
        }

        if line.starts_with('+') && !line.starts_with("+++") {
            code_lines.push(line[1..].to_string());
        } else if line.starts_with('-') && !line.starts_with("---") {
            continue;
        } else if !line.starts_with('\\') {
            code_lines.push(line.to_string());
        }
    }

    code_lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url_ssh() {
        let url = "git@github.com:chmp/markdown-app.git";
        let result = parse_github_url(url).unwrap();
        assert_eq!(result, ("chmp".to_string(), "markdown-app".to_string()));
    }

    #[test]
    fn test_parse_github_url_https() {
        let url = "https://github.com/chmp/markdown-app.git";
        let result = parse_github_url(url).unwrap();
        assert_eq!(result, ("chmp".to_string(), "markdown-app".to_string()));
    }

    #[test]
    fn test_parse_github_url_https_no_git() {
        let url = "https://github.com/chmp/markdown-app";
        let result = parse_github_url(url).unwrap();
        assert_eq!(result, ("chmp".to_string(), "markdown-app".to_string()));
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("file.rs"), "rust");
        assert_eq!(detect_language("file.py"), "python");
        assert_eq!(detect_language("file.md"), "markdown");
        assert_eq!(detect_language("path/to/file.js"), "javascript");
        assert_eq!(detect_language("unknown.xyz"), "");
    }

    #[test]
    fn test_comment_syntax() {
        assert_eq!(get_comment_prefix("rust"), "//");
        assert_eq!(get_comment_prefix("python"), "#");
        assert_eq!(get_comment_prefix("markdown"), "<!--");

        assert_eq!(get_comment_suffix("rust"), "");
        assert_eq!(get_comment_suffix("python"), "");
        assert_eq!(get_comment_suffix("markdown"), " -->");
    }

    #[test]
    fn test_extract_code_from_diff_hunk() {
        let diff_hunk = r#"@@ -55,6 +59,8 @@ pub struct BuildConfig {
     pub output: Option<PathBuf>,
     pub document_store: ObjectStore,
     pub object_stores: BTreeMap<String, ObjectStore>,
+    /// HTML sanitization configuration
+    pub sanitizer: SanitizerConfig,"#;

        let result = extract_code_from_diff_hunk(diff_hunk);

        assert!(result.iter().any(|l| l.contains("pub output: Option<PathBuf>")));
        assert!(result.iter().any(|l| l.contains("HTML sanitization configuration")));
        assert!(result.iter().any(|l| l.contains("pub sanitizer: SanitizerConfig")));
    }

    #[test]
    fn test_parse_comments_json_with_sample_data() {
        let json = std::fs::read_to_string("../tmp_review.json")
            .or_else(|_| std::fs::read_to_string("tmp_review.json"))
            .expect("Failed to read tmp_review.json - make sure it exists");

        let comments = parse_comments_json(&json).expect("Failed to parse JSON");

        assert!(!comments.is_empty(), "Expected at least one comment");

        let first_comment = &comments[0];
        assert!(!first_comment.path.is_empty());
        assert!(!first_comment.body.is_empty());
        assert!(!first_comment.diff_hunk.is_empty());
        assert!(!first_comment.user.login.is_empty());
    }

    #[test]
    fn test_format_comments_as_markdown() {
        let json = std::fs::read_to_string("../tmp_review.json")
            .or_else(|_| std::fs::read_to_string("tmp_review.json"))
            .expect("Failed to read tmp_review.json");

        let comments = parse_comments_json(&json).expect("Failed to parse JSON");
        let grouped = group_comments_by_file(comments);
        let markdown = format_comments_as_markdown(grouped);

        assert!(markdown.contains("##"));
        assert!(markdown.contains("```"));
        assert!(markdown.contains("<review user="));
        assert!(markdown.contains("</review>"));
    }

    #[test]
    #[ignore]
    fn test_show_formatted_output() {
        let json = std::fs::read_to_string("../tmp_review.json")
            .or_else(|_| std::fs::read_to_string("tmp_review.json"))
            .expect("Failed to read tmp_review.json");

        let comments = parse_comments_json(&json).expect("Failed to parse JSON");
        let grouped = group_comments_by_file(comments);
        let markdown = format_comments_as_markdown(grouped);

        println!("\n{}", markdown);
    }

    // Integration tests with examples
    fn test_formatting(json: &str, expected: &str) {
        let comments = parse_comments_json(json).expect("Failed to parse JSON");
        let grouped = group_comments_by_file(comments);
        let output = format_comments_as_markdown(grouped);
        assert_eq!(output.trim(), expected.trim(), "Output mismatch:\n\nGot:\n{}\n\nExpected:\n{}", output, expected);
    }

    #[test]
    fn test_example_simple_comment() {
        let json = include_str!("../../examples/simple_comment.json");
        let expected = include_str!("../../examples/simple_comment.expected.md");
        test_formatting(json, expected);
    }

    #[test]
    fn test_example_multiple_comments() {
        let json = include_str!("../../examples/multiple_comments.json");
        let expected = include_str!("../../examples/multiple_comments.expected.md");
        test_formatting(json, expected);
    }

    #[test]
    fn test_example_multiline_comment() {
        let json = include_str!("../../examples/multiline_comment.json");
        let expected = include_str!("../../examples/multiline_comment.expected.md");
        test_formatting(json, expected);
    }

    #[test]
    fn test_example_no_line_number() {
        let json = include_str!("../../examples/no_line_number.json");
        let expected = include_str!("../../examples/no_line_number.expected.md");
        test_formatting(json, expected);
    }
}
