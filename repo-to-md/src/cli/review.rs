use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use argh::FromArgs;

use crate::{
    client::{
        Comment, FetchReviewCommentsClient, GetCurrentUserClient, ListPullRequestsClient,
        ListReviewsClient, Review,
    },
    formatting::format_comments_as_markdown,
    repository::{GetCurrentBranch, GetRepoistoryInfo},
};

#[derive(FromArgs)]
#[argh(subcommand, name = "review")]
/// Fetch and format PR review comments as markdown
pub struct ReviewCommand {
    /// PR number to fetch comments from (not needed if --json-file is provided)
    #[argh(option)]
    pub pr: Option<u32>,

    /// repository name (auto-detected from git remote if not provided)
    #[argh(option)]
    pub repo: Option<String>,

    /// specific the review either an ID or an index (skips interactive selection)
    #[argh(option)]
    pub review: Option<String>,

    /// filter reviews by author login (use "@me" for your own reviews)
    #[argh(option)]
    pub author: Option<String>,
}

/// Handles the review subcommand - fetches and formats PR comments.
///
/// # Arguments
///
/// * `cmd` - The review command parameters
///
/// # Returns
///
/// Ok(()) on successful execution
///
/// # Errors
///
/// Returns an error if:
/// - Neither PR ID nor JSON file is provided
/// - Repository info cannot be auto-detected
/// - API calls fail
/// - File operations fail
impl ReviewCommand {
    pub fn run(
        self,
        client: &(impl GetCurrentUserClient
              + ListReviewsClient
              + FetchReviewCommentsClient
              + ListPullRequestsClient),
        repository: &(impl GetRepoistoryInfo + GetCurrentBranch),
    ) -> Result<()> {
        let review_id = self.get_review_id(client, repository)?;
        let comments = client.fetch_review_comments(&review_id)?;

        if comments.is_empty() {
            eprintln!("No comments found");
            return Ok(());
        }

        let grouped_comments = group_comments_by_file(comments);
        let markdown = format_comments_as_markdown(grouped_comments);
        print!("{}", markdown);

        Ok(())
    }

    fn get_review_id(
        &self,
        client: &(impl GetCurrentUserClient + ListReviewsClient + ListPullRequestsClient),
        repository: &(impl GetRepoistoryInfo + GetCurrentBranch),
    ) -> Result<String> {
        let review = self.review.as_ref().map(|s| parse_review_id_or_index(s));
        if let Some(ReviewIdOrIndex::Id(review_id)) = review {
            return Ok(review_id.to_string());
        }
        let (owner, repo) = self.get_owner_and_repo(repository)?;
        let pr_number = self.get_pr_number(client, repository, &owner, &repo)?;

        let all_reviews = client.list_reviews(&owner, &repo, pr_number)?;

        // Apply author filter if specified
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
                .context("Failed to auto-detect repository. Please provide --owner and --repo")
        }
    }

    fn get_pr_number(
        &self,
        client: &impl ListPullRequestsClient,
        repository: &impl GetCurrentBranch,
        owner: &str,
        repo: &str,
    ) -> Result<u32> {
        if let Some(number) = self.pr {
            return Ok(number);
        }

        let branch = repository.get_upstream_branch()?;
        let (_, branch) = branch.split_once('/').unwrap_or(("", &branch));
        eprintln!("Branch {branch}");

        let open_prs = client.list_pull_requests(owner, repo)?;

        for pr in open_prs {
            eprintln!("Upstream {}", pr.head_ref_name);
            if pr.head_ref_name == branch {
                return Ok(pr.number);
            }
        }

        bail!("Could not determine PR number");
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

/// Filters reviews by author login (case-insensitive).
///
/// # Arguments
///
/// * `reviews` - All reviews to filter
/// * `author` - The author login to filter by
///
/// # Returns
///
/// A vector of reviews by the specified author
pub(crate) fn filter_reviews_by_author<'a>(reviews: &'a [Review], author: &str) -> Vec<&'a Review> {
    reviews
        .iter()
        .filter(|r| r.author.login.eq_ignore_ascii_case(author))
        .collect()
}

/// Resolves the author filter, handling the special "@me" value.
///
/// If the author is "@me" (case-insensitive), fetches the currently authenticated
/// user's login from GitHub. Otherwise, returns the author string as-is.
///
/// # Arguments
///
/// * `author` - The author filter string (may be "@me")
/// * `client` - GitHub client to fetch current user if needed
///
/// # Returns
///
/// The resolved author login
///
/// # Errors
///
/// Returns an error if "@me" is used but the current user cannot be fetched
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

/// Selects a review by index (1-indexed, or -1 for last).
///
/// # Arguments
///
/// * `reviews` - Reviews to select from
/// * `index` - 1-indexed position (or -1 for last)
///
/// # Returns
///
/// The selected review
///
/// # Errors
///
/// Returns an error if the index is out of bounds or invalid
pub(crate) fn select_review_by_index<'a>(reviews: &[&'a Review], index: i32) -> Result<&'a Review> {
    if reviews.is_empty() {
        anyhow::bail!("No reviews available to select from");
    }

    let selected = if index == -1 {
        *reviews.last().unwrap()
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

/// Groups comments by their file path.
///
/// Takes a vector of comments and organizes them into a HashMap where the key
/// is the file path and the value is a vector of all comments for that file.
///
/// # Arguments
///
/// * `comments` - A vector of [`Comment`] structs to group
///
/// # Returns
///
/// A HashMap mapping file paths to vectors of comments for that file.
pub fn group_comments_by_file(comments: Vec<Comment>) -> HashMap<String, Vec<Comment>> {
    let mut grouped: HashMap<String, Vec<Comment>> = HashMap::new();
    for comment in comments {
        grouped
            .entry(comment.path.clone())
            .or_default()
            .push(comment);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use crate::client::{CommentCount, MockGitHubClient, User};

    use super::*;

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
        // Non-@me values should pass through unchanged
        let client = MockGitHubClient::new("unused");
        let result = resolve_author_filter("alice", &client).unwrap();
        assert_eq!(result, "alice");
    }

    #[test]
    fn test_resolve_author_filter_lowercase_me() {
        let client = MockGitHubClient::new("testuser");
        let result = resolve_author_filter("@me", &client).unwrap();
        assert_eq!(result, "testuser");
    }

    #[test]
    fn test_resolve_author_filter_uppercase_me() {
        let client = MockGitHubClient::new("testuser");
        let result = resolve_author_filter("@ME", &client).unwrap();
        assert_eq!(result, "testuser");
    }

    #[test]
    fn test_resolve_author_filter_mixed_case_me() {
        let client = MockGitHubClient::new("testuser");
        let result = resolve_author_filter("@Me", &client).unwrap();
        assert_eq!(result, "testuser");

        let result = resolve_author_filter("@mE", &client).unwrap();
        assert_eq!(result, "testuser");
    }
}
