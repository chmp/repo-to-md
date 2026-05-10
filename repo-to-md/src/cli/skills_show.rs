use anyhow::{Result, bail};
use argh::FromArgs;

use crate::skills::SKILLS;

/// Show the content of a skill
#[derive(FromArgs)]
#[argh(subcommand, name = "show")]
pub struct Show {
    /// the name of the string
    #[argh(positional)]
    pub name: Option<String>,
}

impl Show {
    pub fn run(self) -> Result<()> {
        if let Some(name) = self.name {
            for (skill_name, skill_content) in SKILLS {
                if *skill_name == name {
                    println!("{skill_content}");
                    return Ok(());
                }
            }
            bail!("No skill named {name}");
        } else {
            for (name, _) in SKILLS {
                println!("{name}");
            }
            Ok(())
        }
    }
}
