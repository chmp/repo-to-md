use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use argh::FromArgs;

use crate::{
    client::{
        Comment, FetchReviewCommentsClient, GetCurrentUserClient, ListPullRequestsClient,
        ListReviewsClient, Review,
    },
    executable::check_executable,
    formatting::{group_comments_by_file, write_comments_as_markdown},
    local::CommentsFile,
    repository::{GetCurrentBranch, GetRepoistoryInfo},
    side_by_side_diff::SideBySideDiff,
};

/// Fetch and format review comments as markdown
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "format")]
pub struct ReviewFormatCommand {
    /// PR number or existing local comments JSON file
    #[argh(positional)]
    pub pr_or_file: Option<PathBuf>,

    /// repository name (auto-detected from git remote if not provided)
    #[argh(option)]
    pub repo: Option<String>,

    /// specific review ID or index (defaults to the last review)
    #[argh(option)]
    pub review: Option<String>,

    /// filter reviews by author login (use "@me" for your own reviews)
    #[argh(option)]
    pub author: Option<String>,

    /// treat the positional argument as a remote review even if a matching file exists
    #[argh(switch)]
    pub remote: bool,
}

impl ReviewFormatCommand {
    pub fn run(
        self,
        client: &(
             impl GetCurrentUserClient
             + ListReviewsClient
             + FetchReviewCommentsClient
             + ListPullRequestsClient
         ),
        repository: &(impl GetRepoistoryInfo + GetCurrentBranch),
        writer: &mut impl Write,
    ) -> Result<()> {
        let comments = if self.should_format_local_review() {
            self.ensure_no_remote_options_for_local_format()?;
            self.fetch_local_comments()?
        } else {
            let review_id = self.get_review_id(client, repository)?;
            client.fetch_review_comments(&review_id)?
        };

        if comments.is_empty() {
            eprintln!("No comments found");
            return Ok(());
        }

        let grouped_comments = group_comments_by_file(comments);
        write_comments_as_markdown(writer, grouped_comments)?;

        Ok(())
    }

    pub fn check_requirements(&self) -> Result<()> {
        if self.should_format_local_review() {
            return Ok(());
        }
        check_executable("gh")?;

        if self.repo.is_none()
            && !matches!(
                self.review.as_deref().map(parse_review_id_or_index),
                Some(ReviewIdOrIndex::Id(_))
            )
        {
            check_executable("git")?;
        }
        Ok(())
    }

    fn should_format_local_review(&self) -> bool {
        !self.remote && self.pr_or_file.as_ref().is_some_and(|path| path.exists())
    }

    fn fetch_local_comments(self) -> Result<Vec<Comment>> {
        let comments_file = self
            .pr_or_file
            .unwrap_or_else(|| PathBuf::from("review-comments.json"));

        let comments_file_content = CommentsFile::from_path(&comments_file).context(format!(
                "Failed to open comments file: {path}. Please run `repo-to-md review local` to create one",
                path = comments_file.display(),
            ))?;

        if comments_file_content.comments.is_empty() {
            return Ok(vec![]);
        }

        let diff = SideBySideDiff::parse(&comments_file_content.raw_diff)?;

        let mut comments = comments_file_content.comments;
        for comment in &mut comments {
            if comment.path == "__global__" || comment.line.is_none() {
                continue;
            }

            if let Some(line) = comment.line
                && let Some(hunk) = diff.find_hunk(&comment.path, line)
            {
                comment.diff_hunk = hunk.to_unified();
            }
        }

        Ok(comments.into_iter().filter(|c| !c.is_minimized).collect())
    }

    fn ensure_no_remote_options_for_local_format(&self) -> Result<()> {
        if self.repo.is_some() || self.review.is_some() || self.author.is_some() || self.remote {
            bail!(
                "Cannot combine local review formatting with remote review options such as --repo, --review, --author, or --remote"
            );
        }

        Ok(())
    }

    fn get_review_id(
        &self,
        client: &(impl GetCurrentUserClient + ListReviewsClient + ListPullRequestsClient),
        repository: &(impl GetRepoistoryInfo + GetCurrentBranch),
    ) -> Result<String> {
        let review = self.review.as_deref().map(parse_review_id_or_index);
        if let Some(ReviewIdOrIndex::Id(review_id)) = review {
            return Ok(review_id.to_string());
        }

        let (owner, repo) = self.get_owner_and_repo(repository)?;
        let pr_number = self.get_pr_number(client, repository, &owner, &repo)?;

        let all_reviews = client.list_reviews(&owner, &repo, pr_number)?;
        let reviews: Vec<&Review> = if let Some(ref author) = self.author {
            let resolved_author = resolve_author_filter(author, client)?;
            let filtered = filter_reviews_by_author(&all_reviews, &resolved_author);
            if filtered.is_empty() {
                anyhow::bail!(
                    "No reviews found by author '{}' for PR #{}",
                    resolved_author,
                    pr_number,
                );
            }
            eprintln!(
                "Filtered to {} review(s) by author '{}'",
                filtered.len(),
                resolved_author
            );
            filtered
        } else {
            all_reviews.iter().collect()
        };

        let index = if let Some(ReviewIdOrIndex::Index(index)) = review {
            index
        } else {
            eprintln!("Select last review");
            -1
        };

        let selected_review = select_review_by_index(&reviews, index)?;

        eprintln!(
            "\nFetching comments from review by {}...\n",
            selected_review.author.login
        );

        Ok(selected_review.id.to_string())
    }

    fn get_owner_and_repo(&self, repository: &impl GetRepoistoryInfo) -> Result<(String, String)> {
        if let Some(repo) = self.repo.as_ref() {
            let Some((owner, repo)) = repo.split_once('/') else {
                bail!("Cannot interpret {repo:?} as 'owner/repo'");
            };

            Ok((owner.to_string(), repo.to_string()))
        } else {
            eprintln!("Determine repository from remotes");
            repository
                .get_github_owner_and_repo()
                .context("Failed to auto-detect repository. Please provide --repo")
        }
    }

    fn get_pr_number(
        &self,
        client: &impl ListPullRequestsClient,
        repository: &impl GetCurrentBranch,
        owner: &str,
        repo: &str,
    ) -> Result<u32> {
        if let Some(pr_number) = self.pr_or_file.as_ref() {
            let pr_number = pr_number.to_string_lossy();
            return pr_number
                .parse::<u32>()
                .with_context(|| format!("Cannot interpret {pr_number:?} as a PR number"));
        }

        let branch = repository.get_upstream_branch()?;
        let (_, branch) = branch.split_once('/').unwrap_or(("", &branch));
        eprintln!("Branch {branch}");

        let open_prs = client.list_pull_requests(owner, repo)?;

        let matching_prs: Vec<_> = open_prs
            .iter()
            .filter(|pr| pr.head_ref_name == branch)
            .collect();

        match <[_; 1]>::try_from(matching_prs) {
            Ok([matching_pr]) => Ok(matching_pr.number),
            Err(matching_prs) if matching_prs.is_empty() => {
                bail!("No open PR found for branch {branch:?}")
            }
            Err(matching_prs) => {
                let pr_list: Vec<String> = matching_prs
                    .iter()
                    .map(|pr| format!("  #{}: {}", pr.number, pr.title))
                    .collect();
                bail!(
                    "Multiple PRs found for branch '{}'. Please select a review by ID:\n{}",
                    branch,
                    pr_list.join("\n")
                );
            }
        }
    }
}

fn parse_review_id_or_index(s: &str) -> ReviewIdOrIndex<'_> {
    if let Ok(index) = s.parse::<i32>() {
        ReviewIdOrIndex::Index(index)
    } else {
        ReviewIdOrIndex::Id(s)
    }
}

enum ReviewIdOrIndex<'s> {
    Id(&'s str),
    Index(i32),
}

pub(crate) fn filter_reviews_by_author<'a>(reviews: &'a [Review], author: &str) -> Vec<&'a Review> {
    reviews
        .iter()
        .filter(|r| r.author.login.eq_ignore_ascii_case(author))
        .collect()
}

fn resolve_author_filter(author: &str, client: &impl GetCurrentUserClient) -> Result<String> {
    if author.eq_ignore_ascii_case("@me") {
        eprintln!("Determine current user");
        let current_user = client
            .get_current_user()
            .context("Failed to get current user. Are you authenticated with gh CLI?")?;
        let current_user = current_user.login;
        eprintln!("Resolved @me to '{}'", current_user);
        Ok(current_user)
    } else {
        Ok(author.to_string())
    }
}

pub(crate) fn select_review_by_index<'a>(reviews: &[&'a Review], index: i32) -> Result<&'a Review> {
    if reviews.is_empty() {
        anyhow::bail!("No reviews available to select from");
    }

    let selected = if index == -1 {
        let Some(last) = reviews.last() else {
            unreachable!("reviews is non-empty");
        };
        *last
    } else if index >= 1 && index as usize <= reviews.len() {
        reviews[(index - 1) as usize]
    } else {
        anyhow::bail!(
            "Review index {} is out of bounds. Valid range: 1-{} or -1 for last review",
            index,
            reviews.len()
        );
    };

    Ok(selected)
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use crate::{
        client::{CommentCount, MockGitHubClient, User},
        repository::MockRepository,
    };

    use super::*;

    fn create_test_review(id: &str, author: &str, comment_count: u32) -> Review {
        Review {
            id: id.to_string(),
            author: User {
                login: author.to_string(),
            },
            state: "APPROVED".to_string(),
            body: Some("Test review".to_string()),
            comments: CommentCount {
                total_count: comment_count,
            },
        }
    }

    #[test]
    fn test_filter_by_author() {
        let reviews = vec![
            create_test_review("1", "alice", 5),
            create_test_review("2", "bob", 3),
            create_test_review("3", "alice", 2),
        ];

        let filtered = filter_reviews_by_author(&reviews, "alice");
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].author.login, "alice");
        assert_eq!(filtered[1].author.login, "alice");
    }

    #[test]
    fn test_filter_case_insensitive() {
        let reviews = vec![create_test_review("1", "Alice", 5)];
        let filtered = filter_reviews_by_author(&reviews, "alice");
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_no_matches() {
        let reviews = vec![create_test_review("1", "alice", 5)];
        let filtered = filter_reviews_by_author(&reviews, "bob");
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_select_by_index_positive() {
        let reviews = [
            create_test_review("1", "alice", 5),
            create_test_review("2", "bob", 3),
        ];
        let review_refs: Vec<&Review> = reviews.iter().collect();

        let selected = select_review_by_index(&review_refs, 1).unwrap();
        assert_eq!(selected.author.login, "alice");

        let selected = select_review_by_index(&review_refs, 2).unwrap();
        assert_eq!(selected.author.login, "bob");
    }

    #[test]
    fn test_select_by_index_last() {
        let reviews = [
            create_test_review("1", "alice", 5),
            create_test_review("2", "bob", 3),
        ];
        let review_refs: Vec<&Review> = reviews.iter().collect();

        let selected = select_review_by_index(&review_refs, -1).unwrap();
        assert_eq!(selected.author.login, "bob");
    }

    #[test]
    fn test_select_out_of_bounds() {
        let reviews = [create_test_review("1", "alice", 5)];
        let review_refs: Vec<&Review> = reviews.iter().collect();

        assert!(select_review_by_index(&review_refs, 0).is_err());
        assert!(select_review_by_index(&review_refs, 2).is_err());
    }

    #[test]
    fn test_select_empty() {
        let reviews: Vec<Review> = vec![];
        let review_refs: Vec<&Review> = reviews.iter().collect();

        assert!(select_review_by_index(&review_refs, 1).is_err());
    }

    #[test]
    fn test_resolve_author_filter_passthrough() {
        let client = MockGitHubClient::new("unused");
        let result = resolve_author_filter("alice", &client).unwrap();
        assert_eq!(result, "alice");
    }

    #[test]
    fn test_resolve_author_filter_me_values() {
        let client = MockGitHubClient::new("testuser");
        assert_eq!(resolve_author_filter("@me", &client).unwrap(), "testuser");
        assert_eq!(resolve_author_filter("@ME", &client).unwrap(), "testuser");
        assert_eq!(resolve_author_filter("@Me", &client).unwrap(), "testuser");
    }

    #[test]
    fn test_format_filters_minimized_comments() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let comments_data = serde_json::json!({
            "version": 1,
            "start_ref": "HEAD~1",
            "start_sha": "abc123",
            "end_ref": "HEAD",
            "end_sha": "def456",
            "raw_diff": "",
            "comments": [
                {
                    "id": "1",
                    "path": "test.rs",
                    "line": 1,
                    "body": "Active comment",
                    "diff_hunk": "",
                    "user": { "login": "user1" },
                    "is_minimized": false
                },
                {
                    "id": "2",
                    "path": "test.rs",
                    "line": 2,
                    "body": "Minimized comment",
                    "diff_hunk": "",
                    "user": { "login": "user2" },
                    "is_minimized": true
                }
            ],
            "viewed_files": []
        });
        temp_file
            .write_all(serde_json::to_string(&comments_data).unwrap().as_bytes())
            .unwrap();
        temp_file.flush().unwrap();

        let mut output = Vec::new();

        let cmd = ReviewFormatCommand {
            pr_or_file: Some(temp_file.path().to_path_buf()),
            ..Default::default()
        };
        cmd.run(
            &MockGitHubClient::new("user"),
            &MockRepository::new("foo", "bar", "baz"),
            &mut output,
        )
        .unwrap();
    }
}
