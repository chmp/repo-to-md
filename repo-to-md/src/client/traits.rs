use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Trait for GitHub API operations
pub trait GetCurrentUserClient {
    /// Fetches the currently authenticated user's login
    fn get_current_user(&self) -> Result<User>;
}

pub trait ListReviewsClient {
    fn list_reviews(&self, owner: &str, repo: &str, pr_number: u32) -> Result<Vec<Review>>;
}

pub trait FetchReviewCommentsClient {
    fn fetch_review_comments(&self, review_id: &str) -> Result<Vec<Comment>>;
}

pub trait ListPullRequestsClient {
    fn list_pull_requests(&self, owner: &str, repo: &str) -> Result<Vec<PullRequest>>;
}

/// A Pull Request review.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Review {
    /// The GitHub node ID of the review
    pub id: String,
    /// The author's login
    pub author: User,
    /// The state of the review (APPROVED, CHANGES_REQUESTED, COMMENTED, etc.)
    pub state: String,
    /// The body/description of the review
    pub body: Option<String>,
    /// Comment count information
    pub comments: CommentCount,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentCount {
    pub total_count: u32,
}

/// A GitHub pull request review comment.
///
/// Represents a single comment on a PR, including the file path, line number,
/// comment text, diff context, and the user who made the comment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Comment {
    /// The comment ID (GitHub node ID for PR comments, UUID for local comments)
    pub id: String,
    /// The file path relative to the repository root
    pub path: String,
    /// The line number in the new version of the file (None for general comments)
    pub line: Option<u32>,
    /// The comment text/content
    pub body: String,
    /// The unified diff hunk showing the code context
    pub diff_hunk: String,
    /// The user who made the comment
    pub user: User,
}

/// A GitHub user.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    /// The user's GitHub login/username
    pub login: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub head_ref_name: String,
}

/// A GitHub issue.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// The GitHub node ID of the issue
    pub id: String,
    /// The issue number
    pub number: u32,
    /// The issue title
    pub title: String,
    /// The issue body/description (can be null)
    pub body: Option<String>,
    /// The author of the issue (can be null if user was deleted)
    pub author: Option<User>,
    /// The state of the issue (OPEN, CLOSED)
    pub state: String,
    /// Labels applied to the issue
    #[serde(default)]
    pub labels: Vec<Label>,
}

/// A GitHub label.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Label {
    /// The label name
    pub name: String,
}

pub trait FetchIssueClient {
    fn fetch_issue(&self, owner: &str, repo: &str, issue_number: u32) -> Result<Issue>;
}
