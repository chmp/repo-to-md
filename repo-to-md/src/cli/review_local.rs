use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
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
pub struct ReviewLocalCommand {
    /// base git ref to compare against (auto-detected if not provided)
    #[argh(positional)]
    pub refs: Vec<String>,

    /// server port (default: 8080, env: REPO_TO_MD_PORT)
    #[argh(option)]
    pub port: Option<u16>,

    /// network address to bind to (default: 127.0.0.1, env: REPO_TO_MD_BIND)
    #[argh(option)]
    pub bind: Option<String>,

    /// JSON file path for comment persistence (default: .review-comments/<base-sha>-<end-sha>.json)
    #[argh(option, short = 'o')]
    pub output: Option<PathBuf>,

    /// do not open browser automatically
    #[argh(switch)]
    pub no_open: bool,

    /// force regeneration of session even if refs have changed or working directory is dirty
    #[argh(switch)]
    pub force: bool,
}

impl ReviewLocalCommand {
    pub fn run(self) -> Result<()> {
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
        let comments_path = match self.output.as_ref() {
            Some(path) => path.clone(),
            None => default_comments_path(&refspec)?,
        };
        let raw_diff = validate_and_prepare_session(&comments_path, &refspec, self.force)?;

        let diff = SideBySideDiff::parse(&raw_diff)?;

        eprintln!("Starting web UI for diff review...");
        eprintln!(
            "  Range: {base}..{end}",
            base = refspec.start_ref,
            end = refspec.end_ref
        );
        eprintln!("  Port: {port}");
        eprintln!("  Comments file: {path}", path = comments_path.display());

        let should_open = !self.no_open;

        tokio::runtime::Runtime::new()
            .context("Failed to create tokio runtime")?
            .block_on(async {
                let server =
                    local::bind_server(refspec, port, comments_path, diff, raw_diff, &bind).await?;

                if should_open {
                    open_url(server.url());
                }

                server.serve().await
            })
    }

    pub fn check_requirements(&self) -> Result<()> {
        check_executable("git")
    }

    fn bind_address(&self) -> Result<String> {
        Ok(self
            .bind
            .clone()
            .or_else(|| std::env::var(BIND_ENV).ok())
            .unwrap_or_else(|| DEFAULT_BIND.to_string()))
    }

    fn port(&self) -> Result<u16> {
        if let Some(port) = self.port {
            return Ok(port);
        }

        let Some(port) = std::env::var(PORT_ENV).ok() else {
            return Ok(DEFAULT_PORT);
        };

        port.parse::<u16>().with_context(|| {
            format!("Failed to parse {PORT_ENV}={port:?} as a valid TCP port number")
        })
    }
}

fn default_comments_path(refspec: &RefSpec) -> Result<PathBuf> {
    default_comments_path_in(Path::new(".review-comments"), refspec)
}

fn default_comments_path_in(directory: &Path, refspec: &RefSpec) -> Result<PathBuf> {
    fs::create_dir_all(directory).with_context(|| {
        format!(
            "Failed to create local review session directory '{path}'",
            path = directory.display()
        )
    })?;

    let gitignore_path = directory.join(".gitignore");
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&gitignore_path)
    {
        Ok(mut file) => file.write_all(b"*.json\n").with_context(|| {
            format!("Failed to write '{path}'", path = gitignore_path.display())
        })?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let contents = fs::read_to_string(&gitignore_path).with_context(|| {
                format!("Failed to read '{path}'", path = gitignore_path.display())
            })?;
            if !contents.lines().any(|line| line.trim() == "*.json") {
                let mut file = OpenOptions::new()
                    .append(true)
                    .open(&gitignore_path)
                    .with_context(|| {
                        format!("Failed to update '{path}'", path = gitignore_path.display())
                    })?;
                if !contents.is_empty() && !contents.ends_with('\n') {
                    file.write_all(b"\n")?;
                }
                file.write_all(b"*.json\n").with_context(|| {
                    format!("Failed to update '{path}'", path = gitignore_path.display())
                })?;
            }
        }
        Err(error) => {
            return Err(error).with_context(|| {
                format!("Failed to create '{path}'", path = gitignore_path.display())
            });
        }
    }

    Ok(directory.join(format!("{}-{}.json", refspec.start_sha, refspec.end_sha)))
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
    use std::sync::Mutex;

    use tempfile::tempdir;

    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn bind_address_prefers_cli() {
        let _guard = ENV_LOCK.lock().expect("lock poisoned");
        set_env(BIND_ENV, Some("127.0.0.2"));
        let command = ReviewLocalCommand {
            bind: Some("0.0.0.0".to_string()),
            ..test_command()
        };

        assert_eq!(command.bind_address().unwrap(), "0.0.0.0");
        set_env(BIND_ENV, None);
    }

    #[test]
    fn bind_address_uses_env_then_default() {
        let _guard = ENV_LOCK.lock().expect("lock poisoned");
        set_env(BIND_ENV, Some("127.0.0.2"));
        assert_eq!(test_command().bind_address().unwrap(), "127.0.0.2");
        set_env(BIND_ENV, None);
        assert_eq!(test_command().bind_address().unwrap(), DEFAULT_BIND);
    }

    #[test]
    fn port_prefers_cli() {
        let _guard = ENV_LOCK.lock().expect("lock poisoned");
        set_env(PORT_ENV, Some("9001"));
        let command = ReviewLocalCommand {
            port: Some(9000),
            ..test_command()
        };

        assert_eq!(command.port().unwrap(), 9000);
        set_env(PORT_ENV, None);
    }

    #[test]
    fn port_uses_env_then_default() {
        let _guard = ENV_LOCK.lock().expect("lock poisoned");
        set_env(PORT_ENV, Some("9001"));
        assert_eq!(test_command().port().unwrap(), 9001);
        set_env(PORT_ENV, None);
        assert_eq!(test_command().port().unwrap(), DEFAULT_PORT);
    }

    #[test]
    fn port_rejects_invalid_env_value() {
        let _guard = ENV_LOCK.lock().expect("lock poisoned");
        set_env(PORT_ENV, Some("not-a-port"));
        let error = test_command().port().unwrap_err();
        assert!(error.to_string().contains("REPO_TO_MD_PORT"));
        set_env(PORT_ENV, None);
    }

    #[test]
    fn default_session_paths_are_keyed_by_commit_shas_and_ignored() {
        let directory = tempdir().unwrap();
        let first_refspec = test_refspec("base-a", "end-a");
        let matching_refspec = test_refspec("base-a", "end-a");
        let changed_refspec = test_refspec("base-a", "end-b");

        let first_path = default_comments_path_in(directory.path(), &first_refspec).unwrap();
        let matching_path = default_comments_path_in(directory.path(), &matching_refspec).unwrap();
        let changed_path = default_comments_path_in(directory.path(), &changed_refspec).unwrap();

        assert_eq!(first_path, matching_path);
        assert_ne!(first_path, changed_path);
        assert_eq!(
            fs::read_to_string(directory.path().join(".gitignore")).unwrap(),
            "*.json\n"
        );
    }

    #[test]
    fn default_session_path_adds_ignore_rule_to_existing_gitignore() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join(".gitignore"), "# local review data\n").unwrap();

        default_comments_path_in(directory.path(), &test_refspec("base", "end")).unwrap();

        assert_eq!(
            fs::read_to_string(directory.path().join(".gitignore")).unwrap(),
            "# local review data\n*.json\n"
        );
    }

    #[test]
    fn matching_default_session_reuses_stored_diff() {
        let directory = tempdir().unwrap();
        let refspec = test_refspec("base-sha", "end-sha");
        let comments_path = default_comments_path_in(directory.path(), &refspec).unwrap();
        let stored_file = CommentsFile {
            version: 1,
            start_ref: "HEAD".to_string(),
            start_sha: "base-sha".to_string(),
            end_ref: "HEAD".to_string(),
            end_sha: "end-sha".to_string(),
            raw_diff: "stored diff".to_string(),
            comments: Vec::new(),
            viewed_files: Vec::new(),
        };
        serde_json::to_writer(fs::File::create(&comments_path).unwrap(), &stored_file).unwrap();

        assert_eq!(
            validate_and_prepare_session(&comments_path, &refspec, false).unwrap(),
            "stored diff"
        );
    }

    fn test_refspec(start_sha: &str, end_sha: &str) -> RefSpec {
        RefSpec {
            start_ref: "HEAD".to_string(),
            end_ref: "HEAD".to_string(),
            start_sha: start_sha.to_string(),
            end_sha: end_sha.to_string(),
        }
    }

    fn test_command() -> ReviewLocalCommand {
        ReviewLocalCommand {
            refs: Vec::new(),
            port: None,
            bind: None,
            output: None,
            no_open: false,
            force: false,
        }
    }

    fn set_env(key: &str, value: Option<&str>) {
        match value {
            Some(value) => unsafe { std::env::set_var(key, value) },
            None => unsafe { std::env::remove_var(key) },
        }
    }
}
