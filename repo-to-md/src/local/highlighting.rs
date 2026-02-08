use std::sync::OnceLock;

use regex::Regex;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

use crate::diff::SideBySideDiff;
use crate::language::detect_language;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static PRE_TAG_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn get_pre_tag_regex() -> &'static Regex {
    PRE_TAG_REGEX.get_or_init(|| {
        // Match <pre style="..."> with any style content
        Regex::new(r#"^<pre style="[^"]*">\n?([\s\S]*?)\n?</pre>\n?$"#)
            .expect("Invalid regex pattern")
    })
}

/// Highlight a block of code, returns Vec of highlighted HTML lines.
///
/// Takes a code block and language identifier, and returns syntax-highlighted
/// HTML for each line. Returns `None` if the language is not recognized or
/// highlighting fails.
pub fn highlight_code(code: &str, lang: &str) -> Option<Vec<String>> {
    let syntax_set = get_syntax_set();

    let syntax = match syntax_set.find_syntax_by_token(lang) {
        Some(s) => s,
        None => {
            // Not an error - just an unrecognized language
            return None;
        }
    };

    let theme_set = ThemeSet::load_defaults();
    let theme = &theme_set.themes["InspiredGitHub"];

    let html = match highlighted_html_for_string(code, syntax_set, syntax, theme) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Syntax highlighting failed for {}: {}", lang, e);
            return None;
        }
    };

    // Use regex to extract content from <pre> wrapper
    let regex = get_pre_tag_regex();
    let inner = match regex.captures(&html) {
        Some(caps) => caps.get(1).map(|m| m.as_str().to_string()),
        None => {
            eprintln!(
                "Unexpected syntect output format for {}. HTML starts with: {}",
                lang,
                &html[..html.len().min(100)]
            );
            return None;
        }
    };

    inner.map(|content| content.lines().map(String::from).collect())
}

impl SideBySideDiff {
    /// Apply syntax highlighting to all files in the diff.
    ///
    /// This method consumes the diff, applies highlighting to each hunk's code lines,
    /// and returns the modified diff with `highlighted_html` populated.
    pub fn highlight(mut self) -> Self {
        for file in &mut self.files {
            let lang = detect_language(&file.path);
            if lang.is_empty() {
                continue;
            }
            for hunk in &mut file.hunks {
                highlight_hunk(hunk, lang);
            }
        }
        self
    }
}

/// Apply syntax highlighting to a single diff hunk.
///
/// Highlights both new-side and old-side code lines, updating the
/// `highlighted_html` field for each line.
fn highlight_hunk(hunk: &mut crate::diff::DiffHunk, lang: &str) {
    // Collect new-side content for highlighting
    let new_lines: Vec<&str> = hunk
        .rows
        .iter()
        .filter_map(|row| row.new_line.as_ref().map(|l| l.content.as_str()))
        .collect();

    let new_code = new_lines.join("\n");
    if let Some(highlighted) = highlight_code(&new_code, lang) {
        // Map highlighted lines back to rows
        let mut hl_iter = highlighted.into_iter();
        for row in &mut hunk.rows {
            if let Some(ref mut line) = row.new_line {
                line.highlighted_html = hl_iter.next();
            }
        }
    }

    // Same for old-side
    let old_lines: Vec<&str> = hunk
        .rows
        .iter()
        .filter_map(|row| row.old_line.as_ref().map(|l| l.content.as_str()))
        .collect();

    let old_code = old_lines.join("\n");
    if let Some(highlighted) = highlight_code(&old_code, lang) {
        let mut hl_iter = highlighted.into_iter();
        for row in &mut hunk.rows {
            if let Some(ref mut line) = row.old_line {
                line.highlighted_html = hl_iter.next();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_code_rust() {
        let code = "fn main() {\n    let x = 42;\n}";
        let result = highlight_code(code, "rust");

        assert!(
            result.is_some(),
            "highlight_code should return Some for valid Rust"
        );

        let lines = result.unwrap();
        assert_eq!(lines.len(), 3, "Should have 3 lines");

        // Verify HTML spans are present (syntax highlighting)
        assert!(
            lines[0].contains("<span"),
            "First line should contain HTML spans for syntax highlighting"
        );

        // Verify the keyword 'fn' is highlighted
        assert!(
            lines[0].contains("fn"),
            "First line should contain 'fn' keyword"
        );
    }

    #[test]
    fn test_highlight_code_unknown_language() {
        let code = "some code";
        let result = highlight_code(code, "unknown_lang_xyz");

        assert!(
            result.is_none(),
            "highlight_code should return None for unknown language"
        );
    }

    #[test]
    fn test_highlight_code_javascript() {
        let code = "const x = 1;";
        let result = highlight_code(code, "javascript");

        assert!(
            result.is_some(),
            "highlight_code should return Some for JavaScript"
        );

        let lines = result.unwrap();
        assert!(
            lines[0].contains("<span"),
            "Should contain HTML spans for syntax highlighting"
        );
    }
}
