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

        assert!(
            result
                .iter()
                .any(|l| l.contains("pub output: Option<PathBuf>"))
        );
        assert!(
            result
                .iter()
                .any(|l| l.contains("HTML sanitization configuration"))
        );
        assert!(
            result
                .iter()
                .any(|l| l.contains("pub sanitizer: SanitizerConfig"))
        );
    }
}

#[cfg(test)]
mod issue_formatting {
    use crate::{client::Issue, formatting::write_issue_as_markdown};

    fn test_issue_formatting(json: &str, expected: &str) {
        let issue = serde_json::from_str::<Issue>(json).expect("Failed to parse JSON");
        let mut actual = Vec::<u8>::new();
        write_issue_as_markdown(&mut actual, &issue).expect("infaillable formatting");

        let actual = String::from_utf8(actual).expect("valid utf8");

        assert_eq!(
            actual.trim(),
            expected.trim(),
            "Output mismatch:\n\nGot:\n{}\n\nExpected:\n{}",
            actual,
            expected
        );
    }

    #[test]
    fn test_simple_issue() {
        let json = include_str!("../../examples/simple_issue.json");
        let expected = include_str!("../../examples/simple_issue.expected.md");
        test_issue_formatting(json, expected);
    }

    #[test]
    fn test_issue_no_body() {
        let json = include_str!("../../examples/issue_no_body.json");
        let expected = include_str!("../../examples/issue_no_body.expected.md");
        test_issue_formatting(json, expected);
    }
}

#[cfg(test)]
mod integration {
    use crate::{
        cli::review::group_comments_by_file, client::Comment,
        formatting::write_comments_as_markdown,
    };

    fn test_formatting(json: &str, expected: &str) {
        let comments = serde_json::from_str::<Vec<Comment>>(json).expect("Failed to parse JSON");
        let grouped = group_comments_by_file(comments);
        let mut output = Vec::<u8>::new();
        write_comments_as_markdown(&mut output, grouped).expect("infallible formatting");
        let output = String::from_utf8(output).expect("valid utf8");
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

#[cfg(test)]
mod cli_end_to_end {
    use crate::{
        cli::{IssueCommand, ReviewCommand},
        client::{
            Comment, CommentCount, FetchReviewCommentsClient, Issue, MockGitHubClient, PullRequest,
            Review, User,
        },
        repository::MockRepository,
    };
    use insta::assert_snapshot;

    impl ReviewCommand {
        fn with_pr(mut self, pr: u32) -> Self {
            self.pr = Some(pr);
            self
        }

        fn with_repo(mut self, repo: &str) -> Self {
            self.repo = Some(repo.to_string());
            self
        }

        fn with_review(mut self, review: &str) -> Self {
            self.review = Some(review.to_string());
            self
        }

        fn with_author(mut self, author: &str) -> Self {
            self.author = Some(author.to_string());
            self
        }

        fn with_apply(mut self) -> Self {
            self.apply = true;
            self
        }

        fn with_force(mut self) -> Self {
            self.force = true;
            self
        }
    }

    impl IssueCommand {
        fn with_issue_number(mut self, number: u32) -> Self {
            self.issue_number = number;
            self
        }

        fn with_repo(mut self, repo: &str) -> Self {
            self.repo = Some(repo.to_string());
            self
        }
    }

    #[test]
    fn test_mock_fetch_comments() {
        let client = MockGitHubClient::new("testuser").with_comments(
            "test-review",
            [Comment {
                path: "test.rs".to_string(),
                line: Some(1),
                body: "test comment".to_string(),
                diff_hunk: "".to_string(),
                user: User {
                    login: "user".to_string(),
                },
            }],
        );

        let comments = client.fetch_review_comments("test-review").unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].body, "test comment");
    }

    fn create_test_review(id: &str, author: &str, comment_count: u32) -> Review {
        Review {
            id: id.to_string(),
            author: User {
                login: author.to_string(),
            },
            state: "APPROVED".to_string(),
            body: Some("Test review".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            comments: CommentCount {
                total_count: comment_count,
            },
        }
    }

    fn create_test_comment(path: &str, line: Option<u32>, body: &str, user: &str) -> Comment {
        Comment {
            path: path.to_string(),
            line,
            body: body.to_string(),
            diff_hunk:
                "@@ -8,4 +8,6 @@\n fn main() {\n     println!(\"Hello\");\n+    let x = 42;\n }\n"
                    .to_string(),
            user: User {
                login: user.to_string(),
            },
        }
    }

    fn create_test_pull_request(id: &str, number: u32, branch: &str) -> PullRequest {
        PullRequest {
            id: id.to_string(),
            number,
            title: format!("PR #{}", number),
            head_ref_name: branch.to_string(),
        }
    }

    #[test]
    fn test_review_command_with_pr_and_review_id() {
        let client = MockGitHubClient::new("testuser")
            .with_reviews(
                "owner",
                "repo",
                42,
                [create_test_review("review-123", "reviewer", 1)],
            )
            .with_comments(
                "review-123",
                [create_test_comment(
                    "src/lib.rs",
                    Some(10),
                    "This should return a Result instead",
                    "reviewer",
                )],
            );
        let repository = MockRepository::new("owner", "repo", "origin/main");

        let cmd = ReviewCommand::default()
            .with_pr(42)
            .with_repo("owner/repo")
            .with_review("review-123");

        let mut output = Vec::new();
        cmd.run(&client, &repository, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert_snapshot!(output_str);
    }

    #[test]
    fn test_review_command_with_author_filter() {
        let client = MockGitHubClient::new("testuser")
            .with_reviews(
                "owner",
                "repo",
                42,
                [
                    create_test_review("review-1", "alice", 1),
                    create_test_review("review-2", "bob", 2),
                    create_test_review("review-3", "alice", 1),
                ],
            )
            .with_comments(
                "review-3",
                [create_test_comment(
                    "src/main.rs",
                    Some(10),
                    "Nice work!",
                    "alice",
                )],
            );
        let repository = MockRepository::new("owner", "repo", "origin/main");

        let cmd = ReviewCommand::default()
            .with_pr(42)
            .with_repo("owner/repo")
            .with_author("alice");

        let mut output = Vec::new();
        cmd.run(&client, &repository, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert_snapshot!(output_str);
    }

    #[test]
    fn test_review_command_with_at_me_author() {
        let client = MockGitHubClient::new("testuser")
            .with_reviews(
                "owner",
                "repo",
                42,
                [
                    create_test_review("review-1", "testuser", 1),
                    create_test_review("review-2", "other", 2),
                ],
            )
            .with_comments(
                "review-1",
                [create_test_comment(
                    "src/lib.rs",
                    Some(10),
                    "My own comment",
                    "testuser",
                )],
            );
        let repository = MockRepository::new("owner", "repo", "origin/main");

        let cmd = ReviewCommand::default()
            .with_pr(42)
            .with_repo("owner/repo")
            .with_author("@me");

        let mut output = Vec::new();
        cmd.run(&client, &repository, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert_snapshot!(output_str);
    }

    #[test]
    fn test_review_command_auto_detect_pr_from_branch() {
        let client = MockGitHubClient::new("testuser")
            .with_pull_requests(
                "owner",
                "repo",
                [
                    create_test_pull_request("pr-1", 10, "feature-a"),
                    create_test_pull_request("pr-2", 20, "feature-b"),
                ],
            )
            .with_reviews(
                "owner",
                "repo",
                20,
                [create_test_review("review-x", "reviewer", 1)],
            )
            .with_comments(
                "review-x",
                [create_test_comment(
                    "src/feature.rs",
                    Some(10),
                    "Auto-detected PR comment",
                    "reviewer",
                )],
            );
        let repository = MockRepository::new("owner", "repo", "origin/feature-b");

        let cmd = ReviewCommand::default().with_repo("owner/repo");

        let mut output = Vec::new();
        cmd.run(&client, &repository, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert_snapshot!(output_str);
    }

    #[test]
    fn test_review_command_no_comments() {
        let client = MockGitHubClient::new("testuser")
            .with_reviews(
                "owner",
                "repo",
                42,
                [create_test_review("review-123", "reviewer", 0)],
            )
            .with_comments("review-123", []);
        let repository = MockRepository::new("owner", "repo", "origin/main");

        let cmd = ReviewCommand::default()
            .with_pr(42)
            .with_repo("owner/repo")
            .with_review("review-123");

        let mut output = Vec::new();
        let result = cmd.run(&client, &repository, &mut output);

        assert!(result.is_ok());
        let output_str = String::from_utf8(output).unwrap();
        assert_eq!(output_str, "");
    }

    #[test]
    fn test_issue_command_simple() {
        let issue = Issue {
            id: "issue-123".to_string(),
            number: 42,
            title: "Fix the bug".to_string(),
            body: Some("This is a bug that needs fixing.".to_string()),
            author: Some(User {
                login: "reporter".to_string(),
            }),
            state: "OPEN".to_string(),
            created_at: "2024-01-15T10:30:00Z".to_string(),
            labels: vec![],
        };

        let client = MockGitHubClient::new("testuser").with_issue("owner", "repo", issue);
        let repository = MockRepository::new("owner", "repo", "origin/main");

        let cmd = IssueCommand::default()
            .with_issue_number(42)
            .with_repo("owner/repo");

        let mut output = Vec::new();
        cmd.run(&client, &repository, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert_snapshot!(output_str);
    }

    #[test]
    fn test_issue_command_auto_detect_repo() {
        let issue = Issue {
            id: "issue-456".to_string(),
            number: 99,
            title: "Feature request".to_string(),
            body: Some("Please add this feature.".to_string()),
            author: Some(User {
                login: "user".to_string(),
            }),
            state: "OPEN".to_string(),
            created_at: "2024-02-01T08:00:00Z".to_string(),
            labels: vec![],
        };

        let client = MockGitHubClient::new("testuser").with_issue("auto-owner", "auto-repo", issue);
        let repository = MockRepository::new("auto-owner", "auto-repo", "origin/main");

        let cmd = IssueCommand::default().with_issue_number(99);

        let mut output = Vec::new();
        cmd.run(&client, &repository, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert_snapshot!(output_str);
    }

    #[test]
    fn test_review_command_fails_with_multiple_prs_for_branch() {
        let client = MockGitHubClient::new("testuser").with_pull_requests(
            "owner",
            "repo",
            [
                create_test_pull_request("pr-1", 10, "feature-branch"),
                create_test_pull_request("pr-2", 20, "feature-branch"),
            ],
        );
        let repository = MockRepository::new("owner", "repo", "origin/feature-branch");

        let cmd = ReviewCommand::default().with_repo("owner/repo");

        let mut output = Vec::new();
        let result = cmd.run(&client, &repository, &mut output);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Multiple PRs found"));
        assert!(err.contains("#10"));
        assert!(err.contains("#20"));
    }

    #[test]
    fn test_review_command_fails_with_no_pr_for_branch() {
        let client = MockGitHubClient::new("testuser").with_pull_requests(
            "owner",
            "repo",
            [create_test_pull_request("pr-1", 10, "other-branch")],
        );
        let repository = MockRepository::new("owner", "repo", "origin/feature-branch");

        let cmd = ReviewCommand::default().with_repo("owner/repo");

        let mut output = Vec::new();
        let result = cmd.run(&client, &repository, &mut output);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No open PR found for branch"));
    }

    #[test]
    fn test_review_command_apply_fails_with_uncommitted_changes() {
        let client = MockGitHubClient::new("testuser")
            .with_reviews(
                "owner",
                "repo",
                42,
                [create_test_review("review-123", "reviewer", 1)],
            )
            .with_comments(
                "review-123",
                [create_test_comment(
                    "src/lib.rs",
                    Some(10),
                    "Fix this",
                    "reviewer",
                )],
            );
        let repository =
            MockRepository::new("owner", "repo", "origin/main").with_uncommitted_changes(true);

        let cmd = ReviewCommand::default()
            .with_pr(42)
            .with_repo("owner/repo")
            .with_review("review-123")
            .with_apply();

        let mut output = Vec::new();
        let result = cmd.run(&client, &repository, &mut output);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("uncommitted changes"));
    }

    #[test]
    fn test_review_command_apply_force_bypasses_safety_check() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("src").join("lib.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "fn main() {\n    let x = 1;\n}\n").unwrap();

        let client = MockGitHubClient::new("testuser")
            .with_reviews(
                "owner",
                "repo",
                42,
                [create_test_review("review-123", "reviewer", 1)],
            )
            .with_comments(
                "review-123",
                [create_test_comment(
                    "src/lib.rs",
                    Some(2),
                    "Consider renaming x",
                    "reviewer",
                )],
            );
        let repository = MockRepository::new("owner", "repo", "origin/main")
            .with_uncommitted_changes(true)
            .with_repo_root(temp_dir.path().to_path_buf());

        let cmd = ReviewCommand::default()
            .with_pr(42)
            .with_repo("owner/repo")
            .with_review("review-123")
            .with_apply()
            .with_force();

        let mut output = Vec::new();
        let result = cmd.run(&client, &repository, &mut output);

        assert!(result.is_ok());
        let output_str = String::from_utf8(output).unwrap();
        assert_snapshot!("apply_force_output", output_str);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_snapshot!("apply_force_file_content", content);
    }

    #[test]
    fn test_review_command_apply_mode() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("src").join("main.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "fn hello() {\n    println!(\"Hello\");\n}\n").unwrap();

        let client = MockGitHubClient::new("testuser")
            .with_reviews(
                "owner",
                "repo",
                42,
                [create_test_review("review-123", "reviewer", 1)],
            )
            .with_comments(
                "review-123",
                [create_test_comment(
                    "src/main.rs",
                    Some(2),
                    "Add error handling",
                    "reviewer",
                )],
            );
        let repository = MockRepository::new("owner", "repo", "origin/main")
            .with_repo_root(temp_dir.path().to_path_buf());

        let cmd = ReviewCommand::default()
            .with_pr(42)
            .with_repo("owner/repo")
            .with_review("review-123")
            .with_apply();

        let mut output = Vec::new();
        cmd.run(&client, &repository, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert_snapshot!("apply_mode_output", output_str);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_snapshot!("apply_mode_file_content", content);
    }
}
