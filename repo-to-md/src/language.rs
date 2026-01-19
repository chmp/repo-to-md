/// Language detection and comment syntax utilities.
/// Detects the programming language from a file path.
///
/// Maps file extensions to markdown language identifiers for syntax highlighting.
///
/// # Arguments
///
/// * `path` - The file path
///
/// # Returns
///
/// A language identifier string (e.g., "rust", "python", "javascript") or an
/// empty string if the language cannot be determined.
pub(crate) fn detect_language(path: &str) -> &str {
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

/// Returns the comment prefix for a given programming language.
///
/// Used to embed review comments as inline code comments in the markdown output.
///
/// # Arguments
///
/// * `language` - The language identifier (from [`detect_language`])
///
/// # Returns
///
/// The comment prefix string (e.g., "//" for most languages, "#" for Python/Bash, "<!--" for HTML)
pub(crate) fn get_comment_prefix(language: &str) -> &str {
    match language {
        "python" | "bash" | "ruby" | "yaml" | "toml" => "#",
        "html" | "markdown" => "<!--",
        _ => "//",
    }
}

/// Returns the comment suffix for a given programming language.
///
/// Most languages don't require a suffix (just a prefix), but some like HTML need
/// a closing delimiter.
///
/// # Arguments
///
/// * `language` - The language identifier (from [`detect_language`])
///
/// # Returns
///
/// The comment suffix string (" -->" for HTML/Markdown, empty string for most languages)
pub(crate) fn get_comment_suffix(language: &str) -> &str {
    match language {
        "html" | "markdown" => " -->",
        _ => "",
    }
}
