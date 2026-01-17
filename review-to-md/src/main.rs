use anyhow::{Context, Result};
use argh::FromArgs;
use review_to_md::*;

#[derive(FromArgs)]
/// Format GitHub PR comments as markdown
struct Cli {
    /// PR number to fetch comments from (not needed if --json-file is provided)
    #[argh(positional)]
    pr_id: Option<u32>,

    /// path to local JSON file containing saved API response
    #[argh(option)]
    json_file: Option<String>,

    /// repository owner (auto-detected from git remote if not provided)
    #[argh(option)]
    owner: Option<String>,

    /// repository name (auto-detected from git remote if not provided)
    #[argh(option)]
    repo: Option<String>,
}

fn main() -> Result<()> {
    let cli: Cli = argh::from_env();

    let comments = if let Some(json_file) = cli.json_file {
        // Read from local JSON file
        read_comments_from_file(&json_file)?
    } else if let Some(pr_id) = cli.pr_id {
        // Fetch from GitHub API
        let (owner, repo) = if let (Some(owner), Some(repo)) = (cli.owner, cli.repo) {
            (owner, repo)
        } else {
            get_repo_info()
                .context("Failed to auto-detect repository. Please provide --owner and --repo")?
        };
        fetch_pr_comments(&owner, &repo, pr_id)?
    } else {
        anyhow::bail!("Either provide PR_ID or --json-file");
    };

    if comments.is_empty() {
        eprintln!("No comments found");
        return Ok(());
    }

    let grouped_comments = group_comments_by_file(comments);
    let markdown = format_comments_as_markdown(grouped_comments);
    print!("{}", markdown);

    Ok(())
}
