use std::sync::OnceLock;

use regex::Regex;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

use crate::language::detect_language;
use crate::side_by_side_diff::{
    FileStatus, LineStatus, SideBySideChunk, SideBySideDiff, SideBySideFile, SideBySideLine,
};

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

#[derive(Debug, Clone, serde::Serialize)]
pub struct RenderedSideBySideFile {
    pub from_path: String,
    pub to_path: String,
    pub display_path: String,
    pub previous_path: Option<String>,
    pub status: FileStatus,
    pub chunks: Vec<RenderedSideBySideChunk>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RenderedSideBySideChunk {
    pub from_range: std::ops::Range<usize>,
    pub to_range: std::ops::Range<usize>,
    pub lines: Vec<RenderedSideBySideLine>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RenderedSideBySideLine {
    pub status: LineStatus,
    pub from: String,
    pub to: String,
    pub from_highlighted_html: Option<String>,
    pub to_highlighted_html: Option<String>,
}

pub fn render_diff(diff: &SideBySideDiff<'_>) -> Vec<RenderedSideBySideFile> {
    diff.files.iter().map(render_file).collect()
}

fn render_file(file: &SideBySideFile<'_>) -> RenderedSideBySideFile {
    let lang = detect_language(file.display_path().as_str());
    let chunks = file
        .chunks
        .iter()
        .map(|chunk| render_chunk(chunk, lang))
        .collect();

    RenderedSideBySideFile {
        from_path: file.from_path.as_str().to_string(),
        to_path: file.to_path.as_str().to_string(),
        display_path: file.display_path().as_str().to_string(),
        previous_path: file.previous_path().map(|path| path.as_str().to_string()),
        status: file.status,
        chunks,
    }
}

fn render_chunk(chunk: &SideBySideChunk<'_>, lang: &str) -> RenderedSideBySideChunk {
    let from_highlights = highlighted_lines(
        chunk
            .lines
            .iter()
            .filter(|line| line.status != LineStatus::Added)
            .map(|line| line.from.as_ref()),
        lang,
    );
    let to_highlights = highlighted_lines(
        chunk
            .lines
            .iter()
            .filter(|line| line.status != LineStatus::Removed)
            .map(|line| line.to.as_ref()),
        lang,
    );

    let mut from_highlights = from_highlights.into_iter();
    let mut to_highlights = to_highlights.into_iter();
    let lines = chunk
        .lines
        .iter()
        .map(|line| render_line(line, &mut from_highlights, &mut to_highlights))
        .collect();

    RenderedSideBySideChunk {
        from_range: chunk.from_range.clone(),
        to_range: chunk.to_range.clone(),
        lines,
    }
}

fn highlighted_lines<'a>(lines: impl Iterator<Item = &'a str>, lang: &str) -> Vec<Option<String>> {
    if lang.is_empty() {
        return Vec::new();
    }

    let lines = lines.collect::<Vec<_>>();
    let code = lines.join("\n");
    highlight_code(&code, lang)
        .unwrap_or_default()
        .into_iter()
        .map(Some)
        .collect()
}

fn render_line(
    line: &SideBySideLine<'_>,
    from_highlights: &mut impl Iterator<Item = Option<String>>,
    to_highlights: &mut impl Iterator<Item = Option<String>>,
) -> RenderedSideBySideLine {
    let from_highlighted_html = match line.status {
        LineStatus::Added => None,
        LineStatus::Context | LineStatus::Removed => from_highlights.next().flatten(),
    };
    let to_highlighted_html = match line.status {
        LineStatus::Removed => None,
        LineStatus::Context | LineStatus::Added => to_highlights.next().flatten(),
    };

    RenderedSideBySideLine {
        status: line.status,
        from: line.from.to_string(),
        to: line.to.to_string(),
        from_highlighted_html,
        to_highlighted_html,
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
