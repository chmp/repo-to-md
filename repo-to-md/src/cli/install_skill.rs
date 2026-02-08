use anyhow::{Context, Result};
use argh::FromArgs;

// Embedded skill files for installation
const SKILLS: &[(&str, &str)] = &[
    (
        "review-to-md",
        include_str!("../../../skills/review-to-md/SKILL.md"),
    ),
    (
        "issue-to-md",
        include_str!("../../../skills/issue-to-md/SKILL.md"),
    ),
    (
        "local-review",
        include_str!("../../../skills/local-review/SKILL.md"),
    ),
];

#[derive(FromArgs)]
#[argh(subcommand, name = "install")]
/// Install the repo-to-md skill for Claude Code
pub struct InstallSkillCommand {
    /// install to local project directory (finds project root via .git or .claude)
    #[argh(switch)]
    pub local: bool,
}

impl InstallSkillCommand {
    /// Installs the skill to the appropriate directory.
    ///
    /// # Arguments
    ///
    /// * `local` - If true, installs to local project directory; otherwise installs globally
    ///
    /// # Returns
    ///
    /// Ok(()) on successful installation
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Home directory cannot be determined (global install)
    /// - Project root cannot be found (local install)
    /// - Directory creation fails
    /// - File write fails
    pub fn run(self) -> Result<()> {
        let base_dir = if self.local {
            // Local project installation - find project root
            let project_root = find_project_root()?;
            eprintln!("Found project root: {}", project_root.display());
            project_root.join(".claude/skills")
        } else {
            // Global installation in home directory
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .context("Could not determine home directory")?;
            std::path::PathBuf::from(home).join(".claude/skills")
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

        let location = if self.local {
            "local project"
        } else {
            "global"
        };
        eprintln!("✓ Installed skills to {} directory:", location);
        for dir in installation_directories {
            eprintln!("  {}", dir.display());
        }

        Ok(())
    }
}

/// Finds the project root by walking up from current directory
/// until finding a .git or .claude directory.
///
/// # Returns
///
/// The path to the project root directory
///
/// # Errors
///
/// Returns an error if no .git or .claude directory is found in any parent directory
fn find_project_root() -> Result<std::path::PathBuf> {
    let mut current = std::env::current_dir().context("Failed to get current directory")?;

    loop {
        // Check if .git exists
        if current.join(".git").exists() {
            return Ok(current);
        }

        // Check if .claude exists
        if current.join(".claude").exists() {
            return Ok(current);
        }

        // Move up to parent directory
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => anyhow::bail!(
                "Could not find project root. No .git or .claude directory found in any parent directory."
            ),
        }
    }
}
