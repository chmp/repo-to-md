// GitHub GraphQL API client.
//
// This module handles all interactions with the GitHub GraphQL API via the `gh` CLI.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::{Comment, User};

/// A Pull Request review.
#[derive(Debug, Deserialize, Serialize)]
pub struct Review {
    /// The GitHub node ID of the review
    pub id: String,
    /// The author's login
    pub author: ReviewAuthor,
    /// The state of the review (APPROVED, CHANGES_REQUESTED, COMMENTED, etc.)
    pub state: String,
    /// The body/description of the review
    pub body: Option<String>,
    /// When the review was created
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Comment count information
    pub comments: CommentCount,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReviewAuthor {
    pub login: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentCount {
    pub total_count: u32,
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

/// Trait for GitHub API operations
pub trait GitHubClient {
    /// Fetches the currently authenticated user's login
    fn get_current_user(&self) -> Result<String>;
}

/// Real GitHub client using gh CLI
pub struct GhClient;

impl GitHubClient for GhClient {
    fn get_current_user(&self) -> Result<String> {
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

        Ok(response.data.viewer.login)
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
pub fn list_reviews(owner: &str, repo: &str, pr_number: u32) -> Result<Vec<Review>> {
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
pub fn fetch_review_comments(review_id: &str) -> Result<Vec<Comment>> {
    let query = r#"
        query($reviewId: ID!) {
          node(id: $reviewId) {
            ... on PullRequestReview {
              comments(first: 100) {
                nodes {
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
