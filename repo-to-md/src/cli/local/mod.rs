mod format;
mod review;

use anyhow::Result;
use argh::FromArgs;

pub use format::FormatCommand;
pub use review::ReviewCommand;

/// Commands for reviewing local git diffs
#[derive(FromArgs)]
#[argh(subcommand, name = "local")]
pub struct LocalCommand {
    #[argh(subcommand)]
    pub command: Option<LocalSubcommand>,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum LocalSubcommand {
    Review(ReviewCommand),
    Format(FormatCommand),
}

impl LocalCommand {
    pub fn run(self) -> Result<()> {
        match self.command {
            Some(LocalSubcommand::Review(cmd)) => cmd.run(),
            Some(LocalSubcommand::Format(cmd)) => cmd.run(),
            None => FormatCommand::default().run(),
        }
    }
}
