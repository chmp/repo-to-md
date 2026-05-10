use anyhow::{Context, Result};
use argh::FromArgs;

use crate::skills::SKILLS;

/// Install the skills
#[derive(FromArgs)]
#[argh(subcommand, name = "install")]
pub struct Install {
    /// install to local project directory (finds project root via .git or .agents)
    #[argh(switch)]
    pub local: bool,

    /// custom installation path (overrides default locations)
    #[argh(option)]
    pub path: Option<String>,
}

impl Install {
    pub fn run(self) -> Result<()> {
        let base_dir = if let Some(custom_path) = self.path {
            std::path::PathBuf::from(custom_path)
        } else if self.local {
            let project_root = find_project_root()?;
            eprintln!("Install into project root: {}", project_root.display());
            project_root.join(".agents/skills")
        } else {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .context("Could not determine home directory")?;
            eprintln!("Install into home directory");
            std::path::PathBuf::from(home).join(".agents/skills")
        };

        let mut installation_directories = Vec::new();
        for (skill_name, content) in SKILLS {
            let skill_dir = base_dir.join(skill_name);
            std::fs::create_dir_all(&skill_dir).context(format!(
                "Failed to create directory: {}",
                skill_dir.display()
            ))?;
            std::fs::write(skill_dir.join("SKILL.md"), content).context(format!(
                "Failed to write skill file: {}",
                skill_dir.display()
            ))?;
            installation_directories.push(skill_dir);
        }

        eprintln!("Installed skills");
        Ok(())
    }
}

/// Finds the project root by walking up from current directory
/// until finding a .git or .agents directory.
fn find_project_root() -> Result<std::path::PathBuf> {
    let mut current = std::env::current_dir().context("Failed to get current directory")?;

    loop {
        // Check if .git exists
        if current.join(".git").exists() {
            return Ok(current);
        }

        // Check if .agents exists
        if current.join(".agents").exists() {
            return Ok(current);
        }

        // Move up to parent directory
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => anyhow::bail!(
                "Could not find project root. No .git or .agents directory found in any parent directory."
            ),
        }
    }
}
