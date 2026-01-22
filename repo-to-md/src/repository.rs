use std::process::Command;

use anyhow::{bail, Context, Result};

/// Mock repository for testing
pub struct MockRepository {
    owner: String,
    repo: String,
    upstream_branch: String,
}

impl MockRepository {
    pub fn new(owner: &str, repo: &str, upstream_branch: &str) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            upstream_branch: upstream_branch.to_string(),
        }
    }
}

impl GetRepoistoryInfo for MockRepository {
    fn get_github_owner_and_repo(&self) -> Result<(String, String)> {
        Ok((self.owner.clone(), self.repo.clone()))
    }
}

impl GetCurrentBranch for MockRepository {
    fn get_upstream_branch(&self) -> Result<String> {
        Ok(self.upstream_branch.clone())
    }
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
pub trait GetRepoistoryInfo {
    fn get_github_owner_and_repo(&self) -> Result<(String, String)>;
}

pub trait GetCurrentBranch {
    fn get_upstream_branch(&self) -> Result<String>;
}

pub struct LocalRepository;

impl GetRepoistoryInfo for LocalRepository {
    fn get_github_owner_and_repo(&self) -> Result<(String, String)> {
        let output = Command::new("git")
            .args(["remote"])
            .output()
            .context("Failed to execute git command")?;

        if !output.status.success() {
            bail!("Failed to list git remotes");
        }

        let remotes_str =
            str::from_utf8(&output.stdout).context("Invalid UTF-8 in git remote output")?;

        let remotes: Vec<&str> = remotes_str.lines().collect();

        if remotes.is_empty() {
            bail!("No git remotes configured");
        }

        let mut github_remotes: Vec<(String, String, String)> = Vec::new();
        for remote in remotes {
            let url_output = Command::new("git")
                .args(["remote", "get-url", remote])
                .output()
                .context("Failed to get remote URL")?;

            if url_output.status.success() {
                let url = str::from_utf8(&url_output.stdout)
                    .context("Invalid UTF-8 in git remote URL")?
                    .trim();

                if let Ok((owner, repo)) = parse_github_url(url) {
                    github_remotes.push((remote.to_string(), owner, repo));
                }
            }
        }

        match github_remotes.len() {
            0 => bail!("No GitHub remotes found"),
            1 => {
                let (_, owner, repo) = github_remotes.remove(0);
                Ok((owner, repo))
            }
            _ => {
                let remote_list: Vec<String> = github_remotes
                    .iter()
                    .map(|(name, owner, repo)| format!("  {} -> {}/{}", name, owner, repo))
                    .collect();
                bail!(
                    "Multiple GitHub remotes found. Please specify --repo:\n{}",
                    remote_list.join("\n")
                );
            }
        }
    }
}

impl GetCurrentBranch for LocalRepository {
    fn get_upstream_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "@{upstream}"])
            .output()
            .context("Failed to execute git command")?;

        if !output.status.success() {
            anyhow::bail!(
                "Could not determine upstream branch: {}",
                std::str::from_utf8(&output.stderr).unwrap_or("<invalid UTF8 in stderr>"),
            );
        }

        let upstream = str::from_utf8(&output.stdout)
            .context("Invalid UTF-8 in upstream branch")?
            .trim()
            .to_string();

        if upstream.is_empty() {
            bail!("Could not determine upstream branch: empty result");
        }

        Ok(upstream)
    }
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

    bail!("Could not parse GitHub URL: {}", url)
}

#[cfg(test)]
mod url_parsing {
    use super::parse_github_url;

    #[test]
    fn test_parse_github_url_ssh() {
        let url = "git@github.com:foo/bar-baz.git";
        let result = parse_github_url(url).unwrap();
        assert_eq!(result, ("foo".to_string(), "bar-baz".to_string()));
    }

    #[test]
    fn test_parse_github_url_https() {
        let url = "https://github.com/foo/bar-baz.git";
        let result = parse_github_url(url).unwrap();
        assert_eq!(result, ("foo".to_string(), "bar-baz".to_string()));
    }

    #[test]
    fn test_parse_github_url_https_no_git() {
        let url = "https://github.com/foo/bar-baz";
        let result = parse_github_url(url).unwrap();
        assert_eq!(result, ("foo".to_string(), "bar-baz".to_string()));
    }
}
