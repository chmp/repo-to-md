use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

    /// JSON file path for comment persistence (default depends on review mode)
    #[argh(option, short = 'o')]
    pub output: Option<PathBuf>,

    /// review an existing unified diff file (session keyed by its Git blob ID)
    #[argh(option)]
    pub diff: Option<PathBuf>,

    /// review one commit against its first parent (or the empty tree for a root commit)
    #[argh(option)]
    pub commit: Option<String>,

    /// do not open browser automatically
    #[argh(switch)]
    pub no_open: bool,
}

impl ReviewLocalCommand {
    pub fn run(self) -> Result<()> {
        let bind = self.bind_address()?;
        let port = self.port()?;
        let input = self.review_input()?;

        let (refspec, comments_path, raw_diff, title, parsed_diff) = match input {
            ReviewInput::Refs { base, end } => {
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
                let title = format!("Range: {base}..{end}");
                (refspec, comments_path, raw_diff, title, None)
            }
            ReviewInput::DiffFile(path) => {
                let raw_diff = read_diff_file(&path)?;
                let diff = SideBySideDiff::parse(&raw_diff).with_context(|| {
                    format!("Failed to parse diff file '{path}'", path = path.display())
                })?;
                let blob_id = git_blob_id(raw_diff.as_bytes())?;
                let source_path = path.display().to_string();
                let refspec = RefSpec {
                    start_ref: source_path.clone(),
                    end_ref: String::new(),
                    start_sha: blob_id.clone(),
                    end_sha: String::new(),
                };
                let comments_path = match self.output.as_ref() {
                    Some(path) => path.clone(),
                    None => default_diff_comments_path_in(Path::new(".review-comments"), &blob_id)?,
                };
                let raw_diff =
                    validate_and_prepare_diff_session(&comments_path, &refspec, raw_diff)?;
                let title = format!("Diff file: {source_path}");
                (refspec, comments_path, raw_diff, title, Some(diff))
            }
            ReviewInput::Commit(commit) => {
                let refspec = review_commit_spec(&commit)?;
                let comments_path = match self.output.as_ref() {
                    Some(path) => path.clone(),
                    None => default_comments_path_in(Path::new(".review-comments"), &refspec)?,
                };
                let raw_diff = validate_and_prepare_session(&comments_path, &refspec)?;
                let title = format!("Commit: {commit} (against first parent)");
                (refspec, comments_path, raw_diff, title, None)
            }
        };

        let diff = match parsed_diff {
            Some(diff) => diff,
            None => SideBySideDiff::parse(&raw_diff)?,
        };

        eprintln!("Starting web UI for diff review...");
        eprintln!("  {title}");
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

    fn review_input(&self) -> Result<ReviewInput> {
        match (
            self.diff.as_ref(),
            self.commit.as_deref(),
            self.refs.is_empty(),
        ) {
            (Some(_), Some(_), _) => bail!("--diff and --commit cannot be used together"),
            (Some(_), None, false) => bail!("--diff cannot be combined with positional refs"),
            (None, Some(_), false) => bail!("--commit cannot be combined with positional refs"),
            (Some(path), None, true) => Ok(ReviewInput::DiffFile(path.clone())),
            (None, Some(commit), true) => Ok(ReviewInput::Commit(commit.to_string())),
            (None, None, _) => match self.refs.as_slice() {
                [] => Ok(ReviewInput::Refs {
                    base: detect_base_branch()?,
                    end: String::from("HEAD"),
                }),
                [base] => Ok(ReviewInput::Refs {
                    base: base.clone(),
                    end: String::from("HEAD"),
                }),
                [base, end] => Ok(ReviewInput::Refs {
                    base: base.clone(),
                    end: end.clone(),
                }),
                args => bail!(
                    "Expected at most a base ref and an end ref, got {len} refs",
                    len = args.len()
                ),
            },
        }
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

#[derive(Debug)]
enum ReviewInput {
    Refs { base: String, end: String },
    DiffFile(PathBuf),
    Commit(String),
}

fn default_comments_path_in(directory: &Path, refspec: &RefSpec) -> Result<PathBuf> {
    prepare_comments_directory(directory)?;

    Ok(directory.join(format!("{}-{}.json", refspec.start_sha, refspec.end_sha)))
}

fn default_diff_comments_path_in(directory: &Path, blob_id: &str) -> Result<PathBuf> {
    prepare_comments_directory(directory)?;

    Ok(directory.join(format!("diff-{blob_id}.json")))
}

fn prepare_comments_directory(directory: &Path) -> Result<()> {
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

    Ok(())
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
    generate_raw_diff_in(Path::new("."), refspec)
}

fn generate_raw_diff_in(directory: &Path, refspec: &RefSpec) -> Result<String> {
    let diff_args = refspec.diff_args();
    let output = Command::new("git")
        .arg("diff")
        .arg("--unified=3")
        .args(&diff_args)
        .current_dir(directory)
        .output()
        .context("Failed to execute git diff")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff failed: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn read_diff_file(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("Failed to read diff file '{path}'", path = path.display()))
}

fn git_blob_id(content: &[u8]) -> Result<String> {
    git_blob_id_in(Path::new("."), content)
}

fn git_blob_id_in(directory: &Path, content: &[u8]) -> Result<String> {
    let mut child = Command::new("git")
        .args(["hash-object", "--stdin"])
        .current_dir(directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to execute git hash-object")?;

    let mut stdin = child
        .stdin
        .take()
        .context("Failed to open stdin for git hash-object")?;
    stdin
        .write_all(content)
        .context("Failed to send diff content to git hash-object")?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .context("Failed to read git hash-object output")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git hash-object failed: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn review_commit_spec(commit_ref: &str) -> Result<RefSpec> {
    review_commit_spec_in(Path::new("."), commit_ref)
}

fn review_commit_spec_in(directory: &Path, commit_ref: &str) -> Result<RefSpec> {
    let commit_expression = format!("{commit_ref}^{{commit}}");
    let commit_sha = run_git_in(
        directory,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &commit_expression,
        ],
        "Failed to resolve commit",
    )?;
    let parents = run_git_in(
        directory,
        &["rev-list", "--parents", "-n", "1", &commit_sha],
        "Failed to read commit parents",
    )?;
    let mut parent_tokens = parents.split_whitespace();
    let listed_commit = parent_tokens.next();
    if listed_commit != Some(commit_sha.as_str()) {
        bail!("git rev-list returned an unexpected commit for '{commit_ref}'");
    }

    let base_sha = if let Some(parent) = parent_tokens.next() {
        parent.to_string()
    } else {
        git_empty_tree_id_in(directory)?
    };

    Ok(RefSpec {
        start_ref: base_sha.clone(),
        end_ref: commit_sha.clone(),
        start_sha: base_sha,
        end_sha: commit_sha,
    })
}

fn git_empty_tree_id_in(directory: &Path) -> Result<String> {
    let mut child = Command::new("git")
        .args(["hash-object", "-t", "tree", "--stdin"])
        .current_dir(directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to execute git hash-object for the empty tree")?;
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .context("Failed to read empty tree object ID")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git hash-object for the empty tree failed: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git_in(directory: &Path, args: &[&str], action: &str) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .output()
        .with_context(|| format!("{action}: failed to execute git"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{action}: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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

/// Validate a diff-file session and return the supplied raw diff.
///
/// The caller keys `refspec.start_sha` to the Git blob ID of the supplied diff,
/// so matching sessions represent identical file contents even when the path
/// used to reach the file has changed.
fn validate_and_prepare_diff_session(
    comments_path: &Path,
    refspec: &RefSpec,
    supplied_diff: String,
) -> Result<String> {
    if !comments_path.exists() {
        return Ok(supplied_diff);
    }

    let file = CommentsFile::from_path(comments_path)?;
    let is_matching = file.start_sha == refspec.start_sha && file.end_sha == refspec.end_sha;
    if !is_matching {
        bail!(
            "Session at '{path}' has changed. Choose a different output path with -o, or remove the file to start a new session and discard its comments.",
            path = comments_path.display()
        );
    }

    eprintln!("Resuming diff-file review session");
    Ok(supplied_diff)
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
    fn diff_session_paths_are_keyed_by_blob_id_and_ignored() {
        let directory = tempdir().unwrap();
        let session_dir = directory.path().join(".review-comments");
        let first_path = default_diff_comments_path_in(&session_dir, "blob-a").unwrap();
        let matching_path = default_diff_comments_path_in(&session_dir, "blob-a").unwrap();
        let changed_path = default_diff_comments_path_in(&session_dir, "blob-b").unwrap();

        assert_eq!(first_path, session_dir.join("diff-blob-a.json"));
        assert_eq!(first_path, matching_path);
        assert_ne!(first_path, changed_path);
        assert_eq!(
            fs::read_to_string(session_dir.join(".gitignore")).unwrap(),
            "*\n"
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

    #[test]
    fn matching_diff_session_keeps_current_diff_for_same_blob_id() {
        let directory = tempdir().unwrap();
        let comments_path = directory.path().join("session.json");
        let refspec = test_diff_refspec("source.diff", "blob-id");
        let stored_file = CommentsFile {
            version: 1,
            start_ref: "old-source.diff".to_string(),
            start_sha: "blob-id".to_string(),
            end_ref: String::new(),
            end_sha: String::new(),
            raw_diff: "stored diff".to_string(),
            comments: Vec::new(),
            viewed_files: Vec::new(),
        };
        serde_json::to_writer(fs::File::create(&comments_path).unwrap(), &stored_file).unwrap();

        assert_eq!(
            validate_and_prepare_diff_session(
                &comments_path,
                &refspec,
                "supplied diff".to_string()
            )
            .unwrap(),
            "supplied diff"
        );
    }

    #[test]
    fn changed_diff_content_uses_a_new_session_identity() {
        let directory = tempdir().unwrap();
        let session_dir = directory.path().join(".review-comments");
        let old_path = default_diff_comments_path_in(&session_dir, "old-blob").unwrap();
        let new_path = default_diff_comments_path_in(&session_dir, "new-blob").unwrap();
        let old_refspec = test_diff_refspec("source.diff", "old-blob");
        let new_refspec = test_diff_refspec("source.diff", "new-blob");
        let stored_file = CommentsFile {
            version: 1,
            start_ref: old_refspec.start_ref.clone(),
            start_sha: old_refspec.start_sha.clone(),
            end_ref: String::new(),
            end_sha: String::new(),
            raw_diff: "old diff".to_string(),
            comments: Vec::new(),
            viewed_files: Vec::new(),
        };
        serde_json::to_writer(fs::File::create(&old_path).unwrap(), &stored_file).unwrap();

        assert_ne!(old_path, new_path);
        assert_eq!(
            validate_and_prepare_diff_session(&new_path, &new_refspec, "new diff".to_string())
                .unwrap(),
            "new diff"
        );
    }

    #[test]
    fn diff_session_output_override_protects_comments_when_content_changes() {
        let directory = tempdir().unwrap();
        let comments_path = directory.path().join("session.json");
        let old_refspec = test_diff_refspec("source.diff", "old-blob");
        let stored_file = CommentsFile {
            version: 1,
            start_ref: old_refspec.start_ref,
            start_sha: old_refspec.start_sha,
            end_ref: String::new(),
            end_sha: String::new(),
            raw_diff: "old diff".to_string(),
            comments: Vec::new(),
            viewed_files: Vec::new(),
        };
        serde_json::to_writer(fs::File::create(&comments_path).unwrap(), &stored_file).unwrap();
        let original_contents = fs::read(&comments_path).unwrap();

        let error = validate_and_prepare_diff_session(
            &comments_path,
            &test_diff_refspec("source.diff", "new-blob"),
            "new diff".to_string(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("Choose a different output path"));
        assert_eq!(fs::read(&comments_path).unwrap(), original_contents);
    }

    #[test]
    fn diff_file_reports_missing_and_invalid_input() {
        let directory = tempdir().unwrap();
        let missing_path = directory.path().join("missing.diff");
        let missing_error = read_diff_file(&missing_path).unwrap_err();
        assert!(
            missing_error
                .to_string()
                .contains("Failed to read diff file")
        );

        let invalid_path = directory.path().join("invalid.diff");
        fs::write(&invalid_path, "this is not a unified diff\n").unwrap();
        let raw_diff = read_diff_file(&invalid_path).unwrap();
        let invalid_error = match SideBySideDiff::parse(&raw_diff) {
            Ok(_) => panic!("invalid diff should not parse"),
            Err(error) => error,
        };
        assert!(!invalid_error.to_string().is_empty());
    }

    #[test]
    fn diff_blob_id_is_git_blob_id_for_exact_contents() {
        let directory = tempdir().unwrap();
        run_git_in(
            directory.path(),
            &["init", "--quiet"],
            "Failed to init test repository",
        )
        .unwrap();
        let content = b"diff --git a/file b/file\n";
        let actual = git_blob_id_in(directory.path(), content).unwrap();
        assert_eq!(actual, "a37b34c34cb460e91b2cf77969cf71162c9e2017");
    }

    #[test]
    fn commit_review_uses_root_tree_and_first_parent_for_merges() {
        let directory = tempdir().unwrap();
        run_git_in(
            directory.path(),
            &["init", "--quiet", "-b", "main"],
            "Failed to init test repository",
        )
        .unwrap();

        fs::write(directory.path().join("root.txt"), "root\n").unwrap();
        create_test_commit(directory.path(), "root commit");
        let root_sha = test_git(directory.path(), &["rev-parse", "HEAD"]);
        let root_spec = review_commit_spec_in(directory.path(), &root_sha).unwrap();
        let empty_tree = git_empty_tree_id_in(directory.path()).unwrap();
        let root_diff = generate_raw_diff_in(directory.path(), &root_spec).unwrap();
        assert_eq!(root_spec.start_sha, empty_tree);
        assert_eq!(root_spec.end_sha, root_sha);
        assert!(root_diff.contains("diff --git a/root.txt b/root.txt"));

        fs::write(directory.path().join("root.txt"), "root changed\n").unwrap();
        create_test_commit(directory.path(), "ordinary commit");
        let ordinary_sha = test_git(directory.path(), &["rev-parse", "HEAD"]);
        let ordinary_spec = review_commit_spec_in(directory.path(), &ordinary_sha).unwrap();
        let ordinary_diff = generate_raw_diff_in(directory.path(), &ordinary_spec).unwrap();
        assert_eq!(ordinary_spec.start_sha, root_sha);
        assert!(ordinary_diff.contains("+root changed"));

        test_git(directory.path(), &["checkout", "-b", "feature"]);
        fs::write(directory.path().join("feature.txt"), "feature\n").unwrap();
        create_test_commit(directory.path(), "feature commit");
        test_git(directory.path(), &["checkout", "main"]);
        fs::write(directory.path().join("main.txt"), "main line\n").unwrap();
        create_test_commit(directory.path(), "main commit");
        let first_parent_sha = test_git(directory.path(), &["rev-parse", "HEAD"]);
        test_git(
            directory.path(),
            &[
                "-c",
                "user.name=Test User",
                "-c",
                "user.email=test@example.com",
                "merge",
                "--no-ff",
                "--no-edit",
                "feature",
            ],
        );
        let merge_sha = test_git(directory.path(), &["rev-parse", "HEAD"]);

        let merge_spec = review_commit_spec_in(directory.path(), &merge_sha).unwrap();
        let merge_diff = generate_raw_diff_in(directory.path(), &merge_spec).unwrap();
        assert_eq!(merge_spec.start_sha, first_parent_sha);
        assert!(merge_diff.contains("diff --git a/feature.txt b/feature.txt"));
        assert!(!merge_diff.contains("diff --git a/main.txt b/main.txt"));
    }

    #[test]
    fn review_input_rejects_mode_and_ref_combinations() {
        let diff_and_commit = ReviewLocalCommand {
            diff: Some(PathBuf::from("change.diff")),
            commit: Some("HEAD".to_string()),
            ..test_command()
        };
        assert!(
            diff_and_commit
                .review_input()
                .unwrap_err()
                .to_string()
                .contains("--diff and --commit")
        );

        let diff_and_refs = ReviewLocalCommand {
            diff: Some(PathBuf::from("change.diff")),
            refs: vec!["main".to_string()],
            ..test_command()
        };
        assert!(
            diff_and_refs
                .review_input()
                .unwrap_err()
                .to_string()
                .contains("--diff cannot be combined with positional refs")
        );

        let commit_and_refs = ReviewLocalCommand {
            commit: Some("HEAD".to_string()),
            refs: vec!["main".to_string()],
            ..test_command()
        };
        assert!(
            commit_and_refs
                .review_input()
                .unwrap_err()
                .to_string()
                .contains("--commit cannot be combined with positional refs")
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

    fn test_diff_refspec(source_path: &str, blob_id: &str) -> RefSpec {
        RefSpec {
            start_ref: source_path.to_string(),
            end_ref: String::new(),
            start_sha: blob_id.to_string(),
            end_sha: String::new(),
        }
    }

    fn test_command() -> ReviewLocalCommand {
        ReviewLocalCommand {
            refs: Vec::new(),
            port: None,
            bind: None,
            output: None,
            diff: None,
            commit: None,
            no_open: false,
        }
    }

    fn test_git(directory: &Path, args: &[&str]) -> String {
        run_git_in(directory, args, "Test git command failed").unwrap()
    }

    fn create_test_commit(directory: &Path, message: &str) {
        test_git(directory, &["add", "--all"]);
        test_git(
            directory,
            &[
                "-c",
                "user.name=Test User",
                "-c",
                "user.email=test@example.com",
                "commit",
                "--quiet",
                "-m",
                message,
            ],
        );
    }

    fn set_env(key: &str, value: Option<&str>) {
        match value {
            Some(value) => unsafe { std::env::set_var(key, value) },
            None => unsafe { std::env::remove_var(key) },
        }
    }
}
