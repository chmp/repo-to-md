mod issue;
mod issue_format;
mod query;
pub(crate) mod review;
pub(crate) mod review_format;
mod review_local;
mod skills;
mod skills_install;
mod skills_show;

use anyhow::Result;
use argh::FromArgs;

pub use issue::IssueCommand;
pub use issue_format::IssueFormatCommand;
pub use query::QueryCommand;
pub use review::ReviewCommand;
pub use review_format::ReviewFormatCommand;
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
                cmd.check_requirements()?;
                cmd.run(&GithubClient, &LocalRepository, &mut stdout)
            }
            Command::Issue(cmd) => {
                cmd.check_requirements()?;
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
