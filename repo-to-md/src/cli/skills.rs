use anyhow::Result;
use argh::FromArgs;

use super::{skills_install::Install, skills_show::Show};

/// Commands to interact with skills
#[derive(FromArgs)]
#[argh(subcommand, name = "skills")]
pub struct Skills {
    #[argh(subcommand)]
    pub command: SkillCommand,
}
impl Skills {
    pub fn run(self) -> Result<()> {
        match self.command {
            SkillCommand::Install(cmd) => cmd.run(),
            SkillCommand::Show(cmd) => cmd.run(),
        }
    }
}

/// Skill commands
#[derive(FromArgs)]
#[argh(subcommand)]
pub enum SkillCommand {
    Install(Install),
    Show(Show),
}
