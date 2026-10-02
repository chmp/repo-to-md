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
pub struct ReviewLocalCommand {
    /// base ref and optional end ref; auto-detect base and use HEAD when omitted (max two refs)
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
                "Expected at most a base ref and an end ref, got {len} refs",
                len = args.len()
            ),
        };

        let refspec = RefSpec::parse(&base, &end)?.resolve()?;

        if self.refs.len() < 2 {
            let repo = LocalRepository;
            if let Some(warning) = working_tree_warning(&repo, &refspec)? {
                eprintln!("{warning}");
            }
        }

        let comments_path = match self.output.as_ref() {
            Some(path) => path.clone(),
            None => default_comments_path_in(Path::new(".review-comments"), &refspec)?,
        };
        let raw_diff = validate_and_prepare_session(&comments_path, &refspec)?;

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

fn default_comments_path_in(directory: &Path, refspec: &RefSpec) -> Result<PathBuf> {
    let gitignore_path = directory.join(".gitignore");
    match fs::create_dir(directory) {
        Ok(()) => fs::write(&gitignore_path, "*\n").with_context(|| {
            format!("Failed to write '{path}'", path = gitignore_path.display())
        })?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && directory.is_dir() => {}
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "Failed to create local review session directory '{path}'",
                    path = directory.display()
                )
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

fn working_tree_warning(
    repo: &impl CheckWorkingDirectory,
    refspec: &RefSpec,
) -> Result<Option<String>> {
    if !repo.has_uncommitted_changes()? {
        return Ok(None);
    }

    Ok(Some(format!(
        "Warning: Working directory has uncommitted changes. The review covers committed changes in {base}..{end}; working-tree changes are not included.",
        base = refspec.start_ref,
        end = refspec.end_ref
    )))
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
/// If refs/commits don't match, returns an error to protect existing comments.
fn validate_and_prepare_session(comments_path: &Path, refspec: &RefSpec) -> Result<String> {
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

    bail!(
        "Session at '{path}' has changed. Choose a different output path with -o, or remove the file to start a new session and discard its comments.",
        path = comments_path.display()
    );
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use tempfile::tempdir;

    use super::*;
    use crate::repository::MockRepository;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn rejects_more_than_two_refs_with_a_clear_error() {
        let command = ReviewLocalCommand {
            refs: vec![
                String::from("base"),
                String::from("end"),
                String::from("extra"),
            ],
            port: Some(DEFAULT_PORT),
            bind: Some(DEFAULT_BIND.to_string()),
            ..test_command()
        };

        let error = command.run().unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Expected at most a base ref and an end ref")
        );
    }

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
        let session_dir = directory.path().join(".review-comments");
        let first_refspec = test_refspec("base-a", "end-a");
        let matching_refspec = test_refspec("base-a", "end-a");
        let changed_refspec = test_refspec("base-a", "end-b");

        let first_path = default_comments_path_in(&session_dir, &first_refspec).unwrap();
        let matching_path = default_comments_path_in(&session_dir, &matching_refspec).unwrap();
        let changed_path = default_comments_path_in(&session_dir, &changed_refspec).unwrap();

        assert_eq!(first_path, matching_path);
        assert_ne!(first_path, changed_path);
        assert_eq!(
            fs::read_to_string(session_dir.join(".gitignore")).unwrap(),
            "*\n"
        );
    }

    #[test]
    fn default_session_path_does_not_modify_existing_directory() {
        let directory = tempdir().unwrap();
        let session_dir = directory.path().join(".review-comments");
        fs::create_dir(&session_dir).unwrap();
        let gitignore_path = session_dir.join(".gitignore");
        fs::write(&gitignore_path, "# user rules\n").unwrap();

        default_comments_path_in(&session_dir, &test_refspec("base", "end")).unwrap();

        assert_eq!(
            fs::read_to_string(gitignore_path).unwrap(),
            "# user rules\n"
        );
    }

    #[test]
    fn dirty_working_tree_warning_explains_review_scope() {
        let repo = MockRepository::new("owner", "repo", "main").with_uncommitted_changes(true);
        let mut refspec = test_refspec("base-sha", "end-sha");
        refspec.start_ref = "main".to_string();
        refspec.end_ref = "HEAD".to_string();

        assert_eq!(
            working_tree_warning(&repo, &refspec).unwrap(),
            Some("Warning: Working directory has uncommitted changes. The review covers committed changes in main..HEAD; working-tree changes are not included.".to_string())
        );
    }

    #[test]
    fn changed_session_is_preserved_and_returns_actionable_error() {
        let directory = tempdir().unwrap();
        let comments_path = directory.path().join("session.json");
        let stored_file = CommentsFile {
            version: 1,
            start_ref: "main".to_string(),
            start_sha: "old-base-sha".to_string(),
            end_ref: "HEAD".to_string(),
            end_sha: "old-end-sha".to_string(),
            raw_diff: "stored diff".to_string(),
            comments: Vec::new(),
            viewed_files: Vec::new(),
        };
        serde_json::to_writer(fs::File::create(&comments_path).unwrap(), &stored_file).unwrap();
        let original_contents = fs::read(&comments_path).unwrap();

        let error = validate_and_prepare_session(
            &comments_path,
            &test_refspec("new-base-sha", "new-end-sha"),
        )
        .unwrap_err();

        assert!(error.to_string().contains("Choose a different output path"));
        assert_eq!(fs::read(&comments_path).unwrap(), original_contents);
    }

    #[test]
    fn matching_default_session_reuses_stored_diff() {
        let directory = tempdir().unwrap();
        let refspec = test_refspec("base-sha", "end-sha");
        let session_dir = directory.path().join(".review-comments");
        let comments_path = default_comments_path_in(&session_dir, &refspec).unwrap();
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
            validate_and_prepare_session(&comments_path, &refspec).unwrap(),
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
        }
    }

    fn set_env(key: &str, value: Option<&str>) {
        match value {
            Some(value) => unsafe { std::env::set_var(key, value) },
            None => unsafe { std::env::remove_var(key) },
        }
    }
}
