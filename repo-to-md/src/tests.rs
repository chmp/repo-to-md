mod language_detection {
    use crate::language::detect_language;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("file.rs"), "rust");
        assert_eq!(detect_language("file.py"), "python");
        assert_eq!(detect_language("file.md"), "markdown");
        assert_eq!(detect_language("path/to/file.js"), "javascript");
        assert_eq!(detect_language("unknown.xyz"), "");
    }
}

mod comment_syntax {
    use crate::language::{get_comment_prefix, get_comment_suffix};

    #[test]
    fn test_comment_syntax() {
        assert_eq!(get_comment_prefix("rust"), "//");
        assert_eq!(get_comment_prefix("python"), "#");
        assert_eq!(get_comment_prefix("markdown"), "<!--");

        assert_eq!(get_comment_suffix("rust"), "");
        assert_eq!(get_comment_suffix("python"), "");
        assert_eq!(get_comment_suffix("markdown"), " -->");
    }
}

mod diff_parsing {
    use crate::diff::extract_code_from_diff_hunk;

    #[test]
    fn test_extract_code_from_diff_hunk() {
        let diff_hunk = r#"@@ -55,6 +59,8 @@ pub struct BuildConfig {
     pub output: Option<PathBuf>,
     pub document_store: ObjectStore,
     pub object_stores: BTreeMap<String, ObjectStore>,
+    /// HTML sanitization configuration
+    pub sanitizer: SanitizerConfig,"#;

        let result = extract_code_from_diff_hunk(diff_hunk);

        assert!(result
            .iter()
            .any(|l| l.contains("pub output: Option<PathBuf>")));
        assert!(result
            .iter()
            .any(|l| l.contains("HTML sanitization configuration")));
        assert!(result
            .iter()
            .any(|l| l.contains("pub sanitizer: SanitizerConfig")));
    }
}

#[cfg(test)]
mod integration {
    use crate::{
        cli::review::group_comments_by_file, client::Comment,
        formatting::format_comments_as_markdown,
    };

    fn test_formatting(json: &str, expected: &str) {
        let comments = serde_json::from_str::<Vec<Comment>>(json).expect("Failed to parse JSON");
        let grouped = group_comments_by_file(comments);
        let output = format_comments_as_markdown(grouped);
        assert_eq!(
            output.trim(),
            expected.trim(),
            "Output mismatch:\n\nGot:\n{}\n\nExpected:\n{}",
            output,
            expected
        );
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
