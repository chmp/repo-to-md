/// Diff parsing utilities for handling unified diff format.
/// Number of context lines to show before/after commented lines in large diffs
const CONTEXT_LINES: u32 = 5;

/// Minimum number of lines in a diff hunk before truncation is considered
const MIN_TRUNCATION_THRESHOLD: usize = 20;

/// A line from a diff hunk with its content and line number.
pub(crate) struct DiffLine {
    pub content: String,
    pub new_line_number: Option<u32>,
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
    let mut current_new_line: Option<u32> = None;

    if let Some(first_line) = lines.next() {
        if first_line.starts_with("@@") {
            // Extract starting line number from header like "@@ -55,6 +59,8 @@"
            // We need the number after the '+' sign
            if let Some(plus_pos) = first_line.find('+') {
                let after_plus = &first_line[plus_pos + 1..];
                if let Some(comma_or_space) = after_plus.find([',', ' ']) {
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
                let content = if line.len() > 1 {
                    line[1..].to_string()
                } else {
                    String::new()
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
        } else if let Some('-') = line.chars().next() {
            // Deleted line - include in output but no line number
            if !line.starts_with("---") {
                let content = if line.len() > 1 {
                    line[1..].to_string()
                } else {
                    String::new()
                };
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

/// Extracts code lines from a unified diff hunk.
///
/// Processes a diff hunk to extract only the code lines, excluding diff metadata
/// and deleted lines. Added lines (starting with '+') have the '+' prefix removed.
/// Context lines are included as-is.
///
/// This function is primarily used for testing.
///
/// # Arguments
///
/// * `diff_hunk` - A unified diff hunk string (starting with "@@")
///
/// # Returns
///
/// A vector of code line strings with diff markers removed.
#[cfg(test)]
pub(crate) fn extract_code_from_diff_hunk(diff_hunk: &str) -> Vec<String> {
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
