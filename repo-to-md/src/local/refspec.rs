use std::process::Command;

use anyhow::{Context, Result};

/// Parsed git ref specification with resolved SHAs.
#[derive(Debug, Clone)]
pub struct RefSpec {
    /// Original start ref (e.g., "main", "HEAD~5")
    pub start_ref: String,
    /// Original end ref (e.g., "feature", "HEAD")
    pub end_ref: String,
    /// Resolved SHA for start_ref
    pub start_sha: String,
    /// Resolved SHA for end_ref
    pub end_sha: String,
}

impl RefSpec {
    /// Parse a git-style refspec with explicit start and end refs.
    ///
    /// Both refs are stored directly without resolution.
    /// Call `resolve()` afterward to populate the SHA fields via git.
    pub fn parse(start: &str, end: &str) -> Result<Self> {
        Ok(RefSpec {
            start_ref: start.to_string(),
            end_ref: end.to_string(),
            start_sha: String::new(),
            end_sha: String::new(),
        })
    }

    /// Resolve ref names to SHAs using git rev-parse.
    ///
    /// Consumes self and returns a new RefSpec with populated SHA fields.
    pub fn resolve(mut self) -> Result<Self> {
        self.start_sha = resolve_ref(&self.start_ref)?;
        self.end_sha = resolve_ref(&self.end_ref)?;
        Ok(self)
    }

    /// Get the git diff arguments for this refspec.
    pub fn diff_args(&self) -> Vec<String> {
        vec![self.start_ref.clone(), self.end_ref.clone()]
    }
}

/// Detect the base branch for the current repository.
///
/// Tries in order:
/// 1. `git symbolic-ref refs/remotes/origin/HEAD` (extract branch name)
/// 2. `git rev-parse --verify main`
/// 3. `git rev-parse --verify master`
///
/// Returns an error if none of these succeed.
pub fn detect_base_branch() -> Result<String> {
    // Try symbolic-ref first to get the default branch from origin
    if let Ok(output) = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .output()
        && output.status.success()
    {
        let full_ref = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Extract branch name from "refs/remotes/origin/main"
        if let Some(branch) = full_ref.strip_prefix("refs/remotes/origin/") {
            return Ok(branch.to_string());
        }
    }

    for common in ["main", "master"] {
        if let Ok(output) = Command::new("git")
            .args(["rev-parse", "--verify", common])
            .output()
            && output.status.success()
        {
            return Ok(common.to_string());
        }
    }

    anyhow::bail!(
        "Could not detect base branch. Please specify a base ref explicitly.\n\
         Tried: origin/HEAD, main, master"
    )
}

/// Resolve a git ref to its SHA using `git rev-parse`.
fn resolve_ref(ref_str: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", ref_str])
        .output()
        .context("Failed to execute git rev-parse")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Invalid git ref '{}': {}", ref_str, stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refspec_parse() {
        let refspec = RefSpec::parse("main", "HEAD").unwrap();
        assert_eq!(refspec.start_ref, "main");
        assert_eq!(refspec.end_ref, "HEAD");
        assert!(refspec.start_sha.is_empty());
        assert!(refspec.end_sha.is_empty());
    }

    #[test]
    fn test_refspec_parse_head_tilde() {
        let refspec = RefSpec::parse("HEAD~5", "HEAD~2").unwrap();
        assert_eq!(refspec.start_ref, "HEAD~5");
        assert_eq!(refspec.end_ref, "HEAD~2");
    }

    #[test]
    fn test_refspec_parse_range() {
        let refspec = RefSpec::parse("main", "feature").unwrap();
        assert_eq!(refspec.start_ref, "main");
        assert_eq!(refspec.end_ref, "feature");
    }

    #[test]
    fn test_refspec_diff_args() {
        let refspec = RefSpec {
            start_ref: "main".to_string(),
            end_ref: "feature".to_string(),
            start_sha: "abc123".to_string(),
            end_sha: "def456".to_string(),
        };
        assert_eq!(refspec.diff_args(), vec!["main", "feature"]);
    }
}
