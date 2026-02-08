use std::process::Command;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::client::traits::{
    Comment, FetchIssueClient, FetchReviewCommentsClient, GetCurrentUserClient, Issue, Label,
    ListPullRequestsClient, ListReviewsClient, Review, User,
};

/// Real GitHub client using gh CLI
pub struct GithubClient;

impl GetCurrentUserClient for GithubClient {
    fn get_current_user(&self) -> Result<User> {
        let query = r#"
            query {
              viewer {
                login
              }
            }
        "#;

        let json = run_graphql_query(query, &[])?;

        let response: GraphQLResponse<ViewerData> =
            serde_json::from_str(&json).context("Failed to parse viewer query response")?;

        Ok(User {
            login: response.data.viewer.login,
        })
    }
}

/// Lists all reviews for a pull request.
///
/// Fetches review metadata including ID, author, state, body, and comment count.
/// This is used for interactive review selection.
///
/// # Arguments
///
/// * `owner` - The GitHub repository owner
/// * `repo` - The repository name
/// * `pr_number` - The pull request number
///
/// # Returns
///
/// A vector of [`Review`] structs
///
/// # Errors
///
/// Returns an error if the GraphQL query fails or returns invalid data.
impl ListReviewsClient for GithubClient {
    fn list_reviews(&self, owner: &str, repo: &str, pr_number: u32) -> Result<Vec<Review>> {
        let query = r#"
            query($owner: String!, $repo: String!, $prNumber: Int!) {
              repository(owner: $owner, name: $repo) {
                pullRequest(number: $prNumber) {
                  reviews(first: 20) {
                    nodes {
                      id
                      author { login }
                      state
                      body
                      createdAt
                      comments { totalCount }
                    }
                  }
                }
              }
            }
        "#;

        validate_github_owner(owner)?;
        validate_github_repo(repo)?;

        let pr_number_str = pr_number.to_string();
        let variables = &[
            ("owner", owner),
            ("repo", repo),
            ("prNumber", pr_number_str.as_str()),
        ];

        let json = run_graphql_query(query, variables)?;

        let response: GraphQLResponse<ListReviewsData> =
            serde_json::from_str(&json).context("Failed to parse GraphQL response")?;

        Ok(response.data.repository.pull_request.reviews.nodes)
    }
}

/// Fetches comments from a specific review.
///
/// Retrieves all review comments for the given review ID, including code context
/// and minimization status.
///
/// # Arguments
///
/// * `review_id` - The GitHub node ID of the review (e.g., "PRR_...")
///
/// # Returns
///
/// A vector of [`Comment`] structs
///
/// # Errors
///
/// Returns an error if:
/// - The review ID is invalid
/// - The GraphQL query fails
/// - The response contains invalid data
impl FetchReviewCommentsClient for GithubClient {
    fn fetch_review_comments(&self, review_id: &str) -> Result<Vec<Comment>> {
        let query = r#"
            query($reviewId: ID!) {
              node(id: $reviewId) {
                ... on PullRequestReview {
                  comments(first: 100) {
                    nodes {
                      id
                      body
                      path
                      line
                      startLine
                      diffHunk
                      isMinimized
                      author { login }
                    }
                  }
                }
              }
            }
        "#;

        let variables = &[("reviewId", review_id)];

        let json = run_graphql_query(query, variables)?;

        let response: GraphQLResponse<FetchCommentsData> =
            serde_json::from_str(&json).context("Failed to parse GraphQL response")?;

        let review_node = response
            .data
            .node
            .context("Review not found or invalid review ID")?;

        // Transform GraphQL comments to Comment structs
        let comments = review_node
            .comments
            .nodes
            .into_iter()
            .map(|gc| Comment {
                id: gc.id,
                path: gc.path,
                line: gc.line,
                body: gc.body,
                diff_hunk: gc.diff_hunk,
                user: User {
                    login: gc.author.login,
                },
            })
            .collect();

        Ok(comments)
    }
}

impl ListPullRequestsClient for GithubClient {
    fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<super::traits::PullRequest>> {
        let query = r#"
            query($owner: String!, $repo: String!) {
              repository(owner: $owner, name: $repo) {
                pullRequests(first: 100, states: [OPEN]) {
                  nodes {
                    id
                    number
                    title
                    headRefName
                  }
                }
              }
            }
        "#;

        validate_github_owner(owner)?;
        validate_github_repo(repo)?;

        let variables = &[("owner", owner), ("repo", repo)];

        let json = run_graphql_query(query, variables)?;

        let response: GraphQLResponse<ListPullRequestsData> =
            serde_json::from_str(&json).context("Failed to parse GraphQL response")?;

        Ok(response.data.repository.pull_requests.nodes)
    }
}

impl FetchIssueClient for GithubClient {
    fn fetch_issue(&self, owner: &str, repo: &str, issue_number: u32) -> Result<Issue> {
        let query = r#"
            query($owner: String!, $repo: String!, $issueNumber: Int!) {
              repository(owner: $owner, name: $repo) {
                issue(number: $issueNumber) {
                  id
                  number
                  title
                  body
                  author { login }
                  state
                  createdAt
                  labels(first: 20) { nodes { name } }
                }
              }
            }
        "#;

        validate_github_owner(owner)?;
        validate_github_repo(repo)?;

        let issue_number_str = issue_number.to_string();
        let variables = &[
            ("owner", owner),
            ("repo", repo),
            ("issueNumber", issue_number_str.as_str()),
        ];

        let json = run_graphql_query(query, variables)?;

        let response: GraphQLResponse<FetchIssueData> =
            serde_json::from_str(&json).context("Failed to parse GraphQL response")?;

        let issue_node = response.data.repository.issue.context("Issue not found")?;

        Ok(Issue {
            id: issue_node.id,
            number: issue_node.number,
            title: issue_node.title,
            body: issue_node.body,
            author: issue_node.author.map(|a| User { login: a.login }),
            state: issue_node.state,
            labels: issue_node
                .labels
                .nodes
                .into_iter()
                .map(|l| Label { name: l.name })
                .collect(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListPullRequestsData {
    repository: ListPullRequestsRepository,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListPullRequestsRepository {
    pull_requests: PullRequestsConnection,
}

#[derive(Debug, Deserialize)]
struct PullRequestsConnection {
    nodes: Vec<super::traits::PullRequest>,
}

/// Executes a GraphQL query using the `gh` CLI.
///
/// # Arguments
///
/// * `query` - The GraphQL query string
/// * `variables` - A vector of (name, value) pairs for query variables
///
/// # Returns
///
/// The raw JSON response as a string
///
/// # Errors
///
/// Returns an error if:
/// - The `gh` CLI is not installed or fails to execute
/// - The API request fails
/// - The response contains invalid UTF-8
fn run_graphql_query(query: &str, variables: &[(&str, &str)]) -> Result<String> {
    let query_arg = format!("query={}", query);
    let mut args = vec!["api", "graphql", "-f", &query_arg];

    // Add variables as -F flags
    let var_args: Vec<String> = variables
        .iter()
        .map(|(name, value)| format!("{}={}", name, value))
        .collect();

    for var_arg in &var_args {
        args.push("-F");
        args.push(var_arg);
    }

    let output = Command::new("gh")
        .args(&args)
        .output()
        .context("Failed to execute gh command. Is the gh CLI installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh GraphQL command failed: {}", stderr);
    }

    String::from_utf8(output.stdout).context("Invalid UTF-8 in gh output")
}

// Response structure for fetching review comments
#[derive(Debug, Deserialize)]
struct FetchCommentsData {
    node: Option<ReviewNode>,
}

#[derive(Debug, Deserialize)]
struct ReviewNode {
    comments: CommentsConnection,
}

#[derive(Debug, Deserialize)]
struct CommentsConnection {
    nodes: Vec<GraphQLComment>,
}

// Response structure for fetching the current user
#[derive(Debug, Deserialize)]
struct ViewerData {
    viewer: Viewer,
}

#[derive(Debug, Deserialize)]
struct Viewer {
    login: String,
}

// GraphQL comment structure
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphQLComment {
    id: String,
    body: String,
    path: String,
    line: Option<u32>,
    #[allow(dead_code)]
    start_line: Option<u32>,
    diff_hunk: String,
    #[allow(dead_code)]
    is_minimized: bool,
    author: GraphQLAuthor,
}

#[derive(Debug, Deserialize)]
struct GraphQLAuthor {
    login: String,
}

// GraphQL response wrapper
#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: T,
}

// Response structure for listing reviews
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListReviewsData {
    repository: Repository,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Repository {
    pull_request: PullRequest,
}

#[derive(Debug, Deserialize)]
struct PullRequest {
    reviews: ReviewsConnection,
}

#[derive(Debug, Deserialize)]
struct ReviewsConnection {
    nodes: Vec<Review>,
}

// Response structure for fetching issues
#[derive(Debug, Deserialize)]
struct FetchIssueData {
    repository: IssueRepository,
}

#[derive(Debug, Deserialize)]
struct IssueRepository {
    issue: Option<IssueNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueNode {
    id: String,
    number: u32,
    title: String,
    body: Option<String>,
    author: Option<GraphQLAuthor>,
    state: String,
    labels: LabelConnection,
}

#[derive(Debug, Deserialize)]
struct LabelConnection {
    nodes: Vec<LabelNode>,
}

#[derive(Debug, Deserialize)]
struct LabelNode {
    name: String,
}

fn validate_github_owner(owner: &str) -> Result<()> {
    if owner.is_empty() || owner.len() > 39 {
        anyhow::bail!("GitHub owner must be 1-39 characters");
    }

    if owner.starts_with('-') || owner.ends_with('-') {
        anyhow::bail!("GitHub owner cannot start or end with a hyphen");
    }

    if owner.contains("--") {
        anyhow::bail!("GitHub owner cannot contain consecutive hyphens");
    }

    if !owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        anyhow::bail!("GitHub owner can only contain alphanumeric characters and hyphens");
    }

    Ok(())
}

fn validate_github_repo(repo: &str) -> Result<()> {
    if repo.is_empty() || repo.len() > 100 {
        anyhow::bail!("GitHub repository name must be 1-100 characters");
    }

    if repo.starts_with('.') {
        anyhow::bail!("GitHub repository name cannot start with a dot");
    }

    if !repo
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        anyhow::bail!(
            "GitHub repository name can only contain alphanumeric characters, hyphens, underscores, and dots"
        );
    }

    Ok(())
}

#[cfg(test)]
mod test {
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
