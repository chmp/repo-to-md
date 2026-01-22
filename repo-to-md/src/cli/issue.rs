use std::io::Write;

use anyhow::{bail, Context, Result};
use argh::FromArgs;

use crate::{
    client::FetchIssueClient, formatting::format_issue_as_markdown, repository::GetRepoistoryInfo,
};

#[derive(FromArgs)]
#[argh(subcommand, name = "issue")]
/// Fetch and format GitHub issue as markdown
pub struct IssueCommand {
    /// issue number to fetch
    #[argh(positional)]
    pub issue_number: u32,

    /// repository in owner/repo format (auto-detected from git remote if not provided)
    #[argh(option)]
    pub repo: Option<String>,
}

impl IssueCommand {
    pub fn run(
        self,
        client: &impl FetchIssueClient,
        repository: &impl GetRepoistoryInfo,
        writer: &mut impl Write,
    ) -> Result<()> {
        let (owner, repo) = self.get_owner_and_repo(repository)?;

        eprintln!(
            "Fetching issue #{} from {}/{}...",
            self.issue_number, owner, repo
        );

        let issue = client.fetch_issue(&owner, &repo, self.issue_number)?;
        let markdown = format_issue_as_markdown(&issue);
        write!(writer, "{}", markdown)?;

        Ok(())
    }

    fn get_owner_and_repo(&self, repository: &impl GetRepoistoryInfo) -> Result<(String, String)> {
        if let Some(repo) = self.repo.as_ref() {
            let Some((owner, repo)) = repo.split_once('/') else {
                bail!("Cannot interpret {repo:?} as 'owner/repo'");
            };

            Ok((owner.to_string(), repo.to_string()))
        } else {
            eprintln!("Determine repository from remotes");
            repository
                .get_github_owner_and_repo()
                .context("Failed to auto-detect repository. Please provide --repo")
        }
    }
}
