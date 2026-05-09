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
pub use local::LocalCommand;
pub use query::QueryCommand;
pub use review::ReviewCommand;
pub use skills::Skills;

use crate::{client::GithubClient, executable::check_executable, repository::LocalRepository};

#[derive(FromArgs)]
/// repo-to-md: Format GitHub PR comments as markdown
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
    Local(LocalCommand),
    Skills(Skills),
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let mut stdout = std::io::stdout();
        match self.command {
            Command::Review(cmd) => {
                check_executable("gh")?;
                if cmd.repo.is_none() || cmd.apply {
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
            Command::Local(cmd) => cmd.run(),
            Command::Query(cmd) => {
                check_executable("gh")?;
                cmd.run(&GithubClient)
            }
            Command::Skills(cmd) => cmd.run(),
        }
    }
}
