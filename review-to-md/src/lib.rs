#[cfg(test)]
mod tests;

// Internal modules
mod client;
mod diff;
mod formatting;
mod language;

// Re-export public API
pub use client::{
    fetch_review_comments, list_reviews, CommentCount, GhClient, GitHubClient, Review, ReviewAuthor,
};
pub use formatting::format_comments_as_markdown;

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

/// A GitHub pull request review comment.
///
/// Represents a single comment on a PR, including the file path, line number,
/// comment text, diff context, and the user who made the comment.
#[derive(Debug, Deserialize)]
pub struct Comment {
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
#[derive(Debug, Deserialize)]
pub struct User {
    /// The user's GitHub login/username
    pub login: String,
}

/// Retrieves the GitHub owner and repository name from the git remote URL.
///
/// Executes `git remote get-url origin` to get the remote URL, then parses it
/// to extract the owner and repository name.
///
/// # Returns
///
/// A tuple of `(owner, repo)` on success.
///
/// # Errors
///
/// Returns an error if:
/// - The git command fails to execute
/// - The git remote is not configured
/// - The remote URL is not a valid GitHub URL
pub fn get_repo_info() -> Result<(String, String)> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .context("Failed to execute git command")?;

    if !output.status.success() {
        anyhow::bail!("Git remote not configured");
    }

    let url = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in git remote URL")?
        .trim()
        .to_string();

    parse_github_url(&url)
}

/// Parses a GitHub URL to extract the owner and repository name.
///
/// Supports both SSH and HTTPS GitHub URLs:
/// - SSH: `git@github.com:owner/repo.git`
/// - HTTPS: `https://github.com/owner/repo.git`
///
/// # Arguments
///
/// * `url` - The GitHub URL to parse
///
/// # Returns
///
/// A tuple of `(owner, repo)` on success.
///
/// # Errors
///
/// Returns an error if the URL is not a valid GitHub URL format.
pub(crate) fn parse_github_url(url: &str) -> Result<(String, String)> {
    if let Some(ssh_match) = url.strip_prefix("git@github.com:") {
        let parts: Vec<&str> = ssh_match.trim_end_matches(".git").split('/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    if let Some(https_match) = url.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = https_match.trim_end_matches(".git").split('/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    anyhow::bail!("Could not parse GitHub URL: {}", url)
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
        anyhow::bail!("GitHub repository name can only contain alphanumeric characters, hyphens, underscores, and dots");
    }

    Ok(())
}

/// Fetches pull request review comments from GitHub using the `gh` CLI.
///
/// Executes `gh api` to retrieve all review comments for the specified pull request.
/// The owner and repository name are validated before making the API call.
///
/// # Arguments
///
/// * `owner` - The GitHub repository owner (must be 1-39 alphanumeric characters or hyphens)
/// * `repo` - The repository name (must be 1-100 characters)
/// * `pr_id` - The pull request number
///
/// # Returns
///
/// A vector of [`Comment`] structs representing all review comments on the PR.
///
/// # Errors
///
/// Returns an error if:
/// - The owner or repo names are invalid
/// - The `gh` CLI is not installed or fails to execute
/// - The API request fails (e.g., PR not found, authentication issues)
/// - The response contains invalid JSON or UTF-8
pub fn fetch_pr_comments(owner: &str, repo: &str, pr_id: u32) -> Result<Vec<Comment>> {
    validate_github_owner(owner)?;
    validate_github_repo(repo)?;

    let api_path = format!("/repos/{}/{}/pulls/{}/comments", owner, repo, pr_id);

    let output = Command::new("gh")
        .args([
            "api",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "X-GitHub-Api-Version: 2022-11-28",
            &api_path,
        ])
        .output()
        .context("Failed to execute gh command. Is the gh CLI installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh command failed: {}", stderr);
    }

    let json = String::from_utf8(output.stdout).context("Invalid UTF-8 in gh output")?;
    parse_comments_json(&json)
}

/// Parses a JSON string containing GitHub PR review comments.
///
/// Deserializes the JSON response from the GitHub API into a vector of [`Comment`] structs.
///
/// # Arguments
///
/// * `json` - A JSON string in the format returned by the GitHub PR comments API
///
/// # Returns
///
/// A vector of [`Comment`] structs.
///
/// # Errors
///
/// Returns an error if the JSON is invalid or doesn't match the expected schema.
pub(crate) fn parse_comments_json(json: &str) -> Result<Vec<Comment>> {
    let comments: Vec<Comment> =
        serde_json::from_str(json).context("Failed to parse JSON response from GitHub API")?;
    Ok(comments)
}

/// Reads and parses PR review comments from a JSON file.
///
/// Useful for testing or offline processing. The file should contain JSON in the
/// format returned by the GitHub PR comments API.
///
/// # Arguments
///
/// * `file_path` - Path to the JSON file
///
/// # Returns
///
/// A vector of [`Comment`] structs.
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read
/// - The file contains invalid JSON or doesn't match the expected schema
pub fn read_comments_from_file(file_path: &str) -> Result<Vec<Comment>> {
    let json = std::fs::read_to_string(file_path)
        .context(format!("Failed to read JSON file: {}", file_path))?;
    parse_comments_json(&json)
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
