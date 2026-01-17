use super::*;

mod url_parsing {
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
}

mod language_detection {
    use super::*;

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
    use super::*;

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

mod validation {
    use super::*;

    #[test]
    fn test_validate_github_owner_valid() {
        assert!(validate_github_owner("octocat").is_ok());
        assert!(validate_github_owner("github").is_ok());
        assert!(validate_github_owner("my-org").is_ok());
        assert!(validate_github_owner("user123").is_ok());
        assert!(validate_github_owner("a").is_ok());
        assert!(validate_github_owner("a-b").is_ok());
    }

    #[test]
    fn test_validate_github_owner_invalid() {
        // Empty string
        assert!(validate_github_owner("").is_err());

        // Too long (>39 characters)
        assert!(validate_github_owner(&"a".repeat(40)).is_err());

        // Starts with hyphen
        assert!(validate_github_owner("-start").is_err());

        // Ends with hyphen
        assert!(validate_github_owner("end-").is_err());

        // Consecutive hyphens
        assert!(validate_github_owner("double--dash").is_err());

        // Invalid characters
        assert!(validate_github_owner("user@example").is_err());
        assert!(validate_github_owner("user name").is_err());
        assert!(validate_github_owner("user_name").is_err());
        assert!(validate_github_owner("user.name").is_err());
    }

    #[test]
    fn test_validate_github_repo_valid() {
        assert!(validate_github_repo("hello-world").is_ok());
        assert!(validate_github_repo("my_repo").is_ok());
        assert!(validate_github_repo("repo.name").is_ok());
        assert!(validate_github_repo("test-123_abc.xyz").is_ok());
        assert!(validate_github_repo("a").is_ok());
        assert!(validate_github_repo("123").is_ok());
    }

    #[test]
    fn test_validate_github_repo_invalid() {
        // Empty string
        assert!(validate_github_repo("").is_err());

        // Too long (>100 characters)
        assert!(validate_github_repo(&"a".repeat(101)).is_err());

        // Starts with dot
        assert!(validate_github_repo(".dotfile").is_err());

        // Invalid characters
        assert!(validate_github_repo("repo with spaces").is_err());
        assert!(validate_github_repo("repo@test").is_err());
        assert!(validate_github_repo("repo#test").is_err());
    }
}

mod diff_parsing {
    use super::*;

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

mod integration {
    use super::*;

    fn test_formatting(json: &str, expected: &str) {
        let comments = parse_comments_json(json).expect("Failed to parse JSON");
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
