mod install_skill;
mod issue;
mod query;
pub(crate) mod review;

use anyhow::Result;
use argh::FromArgs;

pub use install_skill::InstallSkillCommand;
pub use issue::IssueCommand;
pub use query::QueryCommand;
pub use review::ReviewCommand;

use crate::{client::GithubClient, repository::LocalRepository};

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
    InstallSkill(InstallSkillCommand),
    Query(QueryCommand),
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let mut stdout = std::io::stdout();
        match self.command {
            Command::Review(cmd) => cmd.run(&GithubClient, &LocalRepository, &mut stdout),
            Command::Issue(cmd) => cmd.run(&GithubClient, &LocalRepository, &mut stdout),
            Command::InstallSkill(cmd) => cmd.run(),
            Command::Query(cmd) => cmd.run(&GithubClient),
        }
    }
}
