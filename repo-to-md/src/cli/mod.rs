mod issue;
mod local;
mod query;
pub(crate) mod review;
mod skills;
mod skills_install;
mod skills_show;

use anyhow::Result;
use argh::FromArgs;

pub use issue::IssueCommand;
pub use query::QueryCommand;
pub use review::{ReviewCommand, ReviewFormatCommand};
pub use skills::Skills;

use crate::{client::GithubClient, executable::check_executable, repository::LocalRepository};

#[derive(FromArgs)]
/// repo-to-md: Format reviews and issues as markdown
pub struct Cli {
    #[argh(subcommand)]
    pub command: Command,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum Command {
    Review(ReviewCommand),
    Issue(IssueCommand),
    Query(QueryCommand),
    Skills(Skills),
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let mut stdout = std::io::stdout();
        match self.command {
            Command::Review(cmd) => {
                if cmd.requires_gh() {
                    check_executable("gh")?;
                }
                if cmd.requires_git() {
                    check_executable("git")?;
                }
                cmd.run(&GithubClient, &LocalRepository, &mut stdout)
            }
            Command::Issue(cmd) => {
                check_executable("gh")?;
                if cmd.repo.is_none() {
                    check_executable("git")?;
                }
                cmd.run(&GithubClient, &LocalRepository, &mut stdout)
            }
            Command::Query(cmd) => {
                check_executable("gh")?;
                cmd.run(&GithubClient)
            }
            Command::Skills(cmd) => cmd.run(),
        }
    }
}
