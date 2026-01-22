// GitHub GraphQL API client.
//
// This module handles all interactions with the GitHub GraphQL API via the `gh` CLI.
mod github;
mod mock;
mod traits;

pub use github::GithubClient;
pub use mock::MockGitHubClient;
pub use traits::{
    Comment, CommentCount, FetchIssueClient, FetchReviewCommentsClient, GetCurrentUserClient,
    Issue, Label, ListPullRequestsClient, ListReviewsClient, PullRequest, Review, User,
};
