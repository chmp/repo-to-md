use std::io::Write;

use anyhow::Result;
use argh::{EarlyExit, FromArgs};

use crate::{
    cli::{review_format::ReviewFormatCommand, review_local::ReviewLocalCommand},
    client::{
        FetchReviewCommentsClient, GetCurrentUserClient, ListPullRequestsClient, ListReviewsClient,
    },
    executable::check_executable,
    repository::{CheckWorkingDirectory, GetCurrentBranch, GetRepoistoryInfo},
};

/// Commands for reviewing GitHub PR comments and local diffs
pub struct ReviewCommand {
    pub command: ReviewSubcommand,
}

/// Commands for reviewing GitHub PR comments and local diffs
#[derive(FromArgs)]
#[argh(subcommand, name = "review")]
struct DerivedReviewCommand {
    #[argh(subcommand)]
    command: ReviewSubcommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum ReviewSubcommand {
    Format(ReviewFormatCommand),
    Local(ReviewLocalCommand),
}

impl ReviewCommand {
    pub fn run(
        self,
        client: &(
             impl GetCurrentUserClient
             + ListReviewsClient
             + FetchReviewCommentsClient
             + ListPullRequestsClient
         ),
        repository: &(impl GetRepoistoryInfo + GetCurrentBranch + CheckWorkingDirectory),
        writer: &mut impl Write,
    ) -> Result<()> {
        match self.command {
            ReviewSubcommand::Format(cmd) => cmd.run(client, repository, writer),
            ReviewSubcommand::Local(cmd) => cmd.run(),
        }
    }

    pub fn check_requirements(&self) -> Result<()> {
        match &self.command {
            ReviewSubcommand::Format(cmd) => {
                if cmd.requires_gh() {
                    check_executable("gh")?;
                }
                if cmd.requires_git() {
                    check_executable("git")?;
                }
            }
            ReviewSubcommand::Local(cmd) => cmd.check_requirements()?,
        }

        Ok(())
    }
}

impl FromArgs for ReviewCommand {
    fn from_args(command_name: &[&str], args: &[&str]) -> std::result::Result<Self, EarlyExit> {
        match args.first().copied() {
            Some("format" | "local" | "help" | "--help") | None => {
                DerivedReviewCommand::from_args(command_name, args).map(|cmd| Self {
                    command: cmd.command,
                })
            }
            Some(_) => {
                let mut command_name = command_name.to_vec();
                command_name.push("format");
                ReviewFormatCommand::from_args(&command_name, args).map(|cmd| ReviewCommand {
                    command: ReviewSubcommand::Format(cmd),
                })
            }
        }
    }

    fn redact_arg_values(
        command_name: &[&str],
        args: &[&str],
    ) -> std::result::Result<Vec<String>, EarlyExit> {
        match args.first().copied() {
            Some("format" | "local" | "help" | "--help") | None => {
                DerivedReviewCommand::redact_arg_values(command_name, args)
            }
            Some(_) => {
                let mut command_name = command_name.to_vec();
                command_name.push("format");
                ReviewFormatCommand::redact_arg_values(&command_name, args)
            }
        }
    }
}

impl argh::SubCommand for ReviewCommand {
    const COMMAND: &'static argh::CommandInfo = DerivedReviewCommand::COMMAND;
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn parse_review_defaults_unknown_subcommand_to_format_argument() {
        let cmd = ReviewCommand::from_args(&["repo-to-md", "review"], &["42"]).unwrap();
        let ReviewSubcommand::Format(format) = cmd.command else {
            panic!("expected review format command");
        };

        assert_eq!(format.pr_or_file, Some(PathBuf::from("42")));
    }

    #[test]
    fn parse_review_keeps_known_local_subcommand() {
        let cmd = ReviewCommand::from_args(&["repo-to-md", "review"], &["local", "main"]).unwrap();
        let ReviewSubcommand::Local(local) = cmd.command else {
            panic!("expected review local command");
        };

        assert_eq!(local.refs, vec![String::from("main")]);
    }
}
