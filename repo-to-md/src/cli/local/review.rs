use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use argh::FromArgs;

use crate::executable::check_executable;
use crate::local::{self, CommentsFile, RefSpec, detect_base_branch};
use crate::repository::{CheckWorkingDirectory, LocalRepository};
use crate::side_by_side_diff::SideBySideDiff;

const DEFAULT_BIND: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8080;
const BIND_ENV: &str = "REPO_TO_MD_BIND";
const PORT_ENV: &str = "REPO_TO_MD_PORT";

/// Launch a web UI for reviewing local git diffs
#[derive(FromArgs)]
#[argh(subcommand, name = "local")]
pub struct LocalCommand {
    /// base git ref to compare against (auto-detected if not provided)
    #[argh(positional)]
    pub refs: Vec<String>,

    /// server port (default: 8080, env: REPO_TO_MD_PORT)
    #[argh(option)]
    pub port: Option<u16>,

    /// network address to bind to (default: 127.0.0.1, env: REPO_TO_MD_BIND)
    #[argh(option)]
    pub bind: Option<String>,

    /// JSON file path for comment persistence (default: review-comments.json)
    #[argh(
        option,
        short = 'o',
        default = "PathBuf::from(\"review-comments.json\")"
    )]
    pub output: PathBuf,

    /// do not open browser automatically
    #[argh(switch)]
    pub no_open: bool,

    /// force regeneration of session even if refs have changed or working directory is dirty
    #[argh(switch)]
    pub force: bool,
}

impl LocalCommand {
    pub fn run(self) -> Result<()> {
        check_executable("git")?;
        let bind = self.bind_address()?;
        let port = self.port()?;

        let (base, end) = match self.refs.as_slice() {
            [] => (detect_base_branch()?, String::from("HEAD")),
            [base] => (base.clone(), String::from("HEAD")),
            [base, end] => (base.clone(), end.clone()),
            args => bail!(
                "Invalid call cannot pass more than two refs, got {len}",
                len = args.len()
            ),
        };

        if self.refs.len() < 2 && !self.force {
            let repo = LocalRepository;
            if repo.has_uncommitted_changes()? {
                bail!(
                    "Working directory has uncommitted changes. \
                     Commit or stash changes before reviewing, or use --force to proceed anyway."
                );
            }
        }

        let refspec = RefSpec::parse(&base, &end)?.resolve()?;
        let raw_diff = validate_and_prepare_session(&self.output, &refspec, self.force)?;

        let diff = SideBySideDiff::parse(&raw_diff)?;

        eprintln!("Starting web UI for diff review...");
        eprintln!(
            "  Range: {base}..{end}",
            base = refspec.start_ref,
            end = refspec.end_ref
        );
        eprintln!("  Port: {port}");
        eprintln!("  Comments file: {path}", path = self.output.display());

        let should_open = !self.no_open;

        tokio::runtime::Runtime::new()
            .context("Failed to create tokio runtime")?
            .block_on(async {
                let server =
                    local::bind_server(refspec, port, self.output, diff, raw_diff, &bind).await?;

                if should_open {
                    open_url(server.url());
                }

                server.serve().await
            })
    }

    fn bind_address(&self) -> Result<String> {
        Ok(resolve_bind_address(
            self.bind.as_deref(),
            std::env::var(BIND_ENV).ok().as_deref(),
        ))
    }

    fn port(&self) -> Result<u16> {
        resolve_port(self.port, std::env::var(PORT_ENV).ok().as_deref())
    }
}

fn resolve_bind_address(cli_bind: Option<&str>, env_bind: Option<&str>) -> String {
    cli_bind.or(env_bind).unwrap_or(DEFAULT_BIND).to_string()
}

fn resolve_port(cli_port: Option<u16>, env_port: Option<&str>) -> Result<u16> {
    if let Some(port) = cli_port {
        return Ok(port);
    }

    let Some(port) = env_port else {
        return Ok(DEFAULT_PORT);
    };

    port.parse::<u16>()
        .with_context(|| format!("Failed to parse {PORT_ENV}={port:?} as a valid TCP port number"))
}

fn open_url(url: &str) {
    if let Err(e) = open::that(url) {
        eprintln!("Failed to open browser: {e}");
    }
}

/// Generate a raw diff from git using the refspec
fn generate_raw_diff(refspec: &RefSpec) -> Result<String> {
    let diff_args = refspec.diff_args();
    let output = Command::new("git")
        .arg("diff")
        .arg("--unified=3")
        .args(&diff_args)
        .output()
        .context("Failed to execute git diff")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff failed: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Validate existing session file and return the raw diff to use.
///
/// If the file exists and refs/commits match, returns the stored diff.
/// If no file exists, generates a new one.
/// If refs/commits don't match, requires --force to regenerate.
fn validate_and_prepare_session(
    comments_path: &Path,
    refspec: &RefSpec,
    force: bool,
) -> Result<String> {
    if !comments_path.exists() {
        eprintln!("Generating diff snapshot...");
        return generate_raw_diff(refspec);
    }

    let file = CommentsFile::from_path(comments_path)?;

    // Check if refs and commits match
    let is_matching = file.start_sha == refspec.start_sha && file.end_sha == refspec.end_sha;

    if is_matching {
        // File exists and matches - use stored diff
        if !file.raw_diff.is_empty() {
            eprintln!("Using existing diff snapshot");
            return Ok(file.raw_diff);
        }
        // Edge case: file exists but no diff stored
        eprintln!("Generating diff snapshot (upgrading file format)...");
        return generate_raw_diff(refspec);
    }

    // Refs or commits don't match
    if force {
        eprintln!("Session has changed Regenerating session.");
        fs::remove_file(comments_path).with_context(|| {
            format!(
                "Failed to delete comments file '{path}'",
                path = comments_path.display()
            )
        })?;
        eprintln!("Generating diff snapshot...");
        return generate_raw_diff(refspec);
    }

    bail!(
        "Session has changed. Use --force to regenerate the session and discard existing comments.",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_address_prefers_cli_then_env_then_default() {
        assert_eq!(
            resolve_bind_address(Some("0.0.0.0"), Some("127.0.0.2")),
            "0.0.0.0"
        );
        assert_eq!(resolve_bind_address(None, Some("127.0.0.2")), "127.0.0.2");
        assert_eq!(resolve_bind_address(None, None), DEFAULT_BIND);
    }

    #[test]
    fn port_prefers_cli_then_env_then_default() {
        assert_eq!(resolve_port(Some(9000), Some("9001")).unwrap(), 9000);
        assert_eq!(resolve_port(None, Some("9001")).unwrap(), 9001);
        assert_eq!(resolve_port(None, None).unwrap(), DEFAULT_PORT);
    }

    #[test]
    fn port_rejects_invalid_env_value() {
        let error = resolve_port(None, Some("not-a-port")).unwrap_err();
        assert!(error.to_string().contains("REPO_TO_MD_PORT"));
    }
}
