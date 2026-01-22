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
mod issue_formatting {
    use crate::{client::Issue, formatting::format_issue_as_markdown};

    fn test_issue_formatting(json: &str, expected: &str) {
        let issue = serde_json::from_str::<Issue>(json).expect("Failed to parse JSON");
        let output = format_issue_as_markdown(&issue);
        assert_eq!(
            output.trim(),
            expected.trim(),
            "Output mismatch:\n\nGot:\n{}\n\nExpected:\n{}",
            output,
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

        let cmd = ReviewCommand {
            pr: Some(42),
            repo: Some("owner/repo".to_string()),
            review: Some("review-123".to_string()),
            author: None,
        };

        let mut output = Vec::new();
        cmd.run(&client, &repository, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("# Pull Request Review Comments"));
        assert!(output_str.contains("`src/lib.rs`"));
        assert!(output_str.contains("This should return a Result instead"));
        assert!(output_str.contains("<review user=\"reviewer\">"));
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

        let cmd = ReviewCommand {
            pr: Some(42),
            repo: Some("owner/repo".to_string()),
            review: None,
            author: Some("alice".to_string()),
        };

        let mut output = Vec::new();
        cmd.run(&client, &repository, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Nice work!"));
        assert!(output_str.contains("<review user=\"alice\">"));
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

        let cmd = ReviewCommand {
            pr: Some(42),
            repo: Some("owner/repo".to_string()),
            review: None,
            author: Some("@me".to_string()),
        };

        let mut output = Vec::new();
        cmd.run(&client, &repository, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("My own comment"));
        assert!(output_str.contains("<review user=\"testuser\">"));
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

        let cmd = ReviewCommand {
            pr: None,
            repo: Some("owner/repo".to_string()),
            review: None,
            author: None,
        };

        let mut output = Vec::new();
        cmd.run(&client, &repository, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Auto-detected PR comment"));
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

        let cmd = ReviewCommand {
            pr: Some(42),
            repo: Some("owner/repo".to_string()),
            review: Some("review-123".to_string()),
            author: None,
        };

        let mut output = Vec::new();
        let result = cmd.run(&client, &repository, &mut output);

        assert!(result.is_ok());
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.is_empty());
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

        let cmd = IssueCommand {
            issue_number: 42,
            repo: Some("owner/repo".to_string()),
        };

        let mut output = Vec::new();
        cmd.run(&client, &repository, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("# Issue #42: Fix the bug"));
        assert!(output_str.contains("This is a bug that needs fixing."));
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

        let cmd = IssueCommand {
            issue_number: 99,
            repo: None,
        };

        let mut output = Vec::new();
        cmd.run(&client, &repository, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("# Issue #99: Feature request"));
        assert!(output_str.contains("Please add this feature."));
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

        let cmd = ReviewCommand {
            pr: None,
            repo: Some("owner/repo".to_string()),
            review: None,
            author: None,
        };

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

        let cmd = ReviewCommand {
            pr: None,
            repo: Some("owner/repo".to_string()),
            review: None,
            author: None,
        };

        let mut output = Vec::new();
        let result = cmd.run(&client, &repository, &mut output);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No open PR found for branch"));
    }
}
