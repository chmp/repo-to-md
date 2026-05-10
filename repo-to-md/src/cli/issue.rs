use std::io::Write;

use anyhow::Result;
use argh::{EarlyExit, FromArgs};

use crate::{
    cli::issue_format::IssueFormatCommand, client::FetchIssueClient, executable::check_executable,
    repository::GetRepoistoryInfo,
};

/// Commands for GitHub issues
pub struct IssueCommand {
    pub command: IssueSubcommand,
}

/// Commands for GitHub issues
#[derive(FromArgs)]
#[argh(subcommand, name = "issue")]
struct DerivedIssueCommand {
    #[argh(subcommand)]
    command: IssueSubcommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum IssueSubcommand {
    Format(IssueFormatCommand),
}

impl IssueCommand {
    pub fn run(
        self,
        client: &impl FetchIssueClient,
        repository: &impl GetRepoistoryInfo,
        writer: &mut impl Write,
    ) -> Result<()> {
        match self.command {
            IssueSubcommand::Format(cmd) => cmd.run(client, repository, writer),
        }
    }

    pub fn check_requirements(&self) -> Result<()> {
        check_executable("gh")?;
        match &self.command {
            IssueSubcommand::Format(cmd) if cmd.requires_git() => check_executable("git")?,
            IssueSubcommand::Format(_) => {}
        }

        Ok(())
    }
}

impl FromArgs for IssueCommand {
    fn from_args(command_name: &[&str], args: &[&str]) -> std::result::Result<Self, EarlyExit> {
        match args.first().copied() {
            Some("format" | "help" | "--help") | None => {
                DerivedIssueCommand::from_args(command_name, args).map(|cmd| Self {
                    command: cmd.command,
                })
            }
            Some(_) => parse_issue_format(command_name, args),
        }
    }

    fn redact_arg_values(
        command_name: &[&str],
        args: &[&str],
    ) -> std::result::Result<Vec<String>, EarlyExit> {
        match args.first().copied() {
            Some("format" | "help" | "--help") | None => {
                DerivedIssueCommand::redact_arg_values(command_name, args)
            }
            Some(_) => {
                let mut format_command_name = command_name.to_vec();
                format_command_name.push("format");
                IssueFormatCommand::redact_arg_values(&format_command_name, args)
            }
        }
    }
}

impl argh::SubCommand for IssueCommand {
    const COMMAND: &'static argh::CommandInfo = DerivedIssueCommand::COMMAND;
}

fn parse_issue_format(
    command_name: &[&str],
    args: &[&str],
) -> std::result::Result<IssueCommand, EarlyExit> {
    let mut format_command_name = command_name.to_vec();
    format_command_name.push("format");
    IssueFormatCommand::from_args(&format_command_name, args).map(|cmd| IssueCommand {
        command: IssueSubcommand::Format(cmd),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_issue_defaults_unknown_subcommand_to_format_argument() {
        let cmd = IssueCommand::from_args(&["repo-to-md", "issue"], &["42"]).unwrap();
        let IssueSubcommand::Format(format) = cmd.command;

        assert_eq!(format.issue_number, 42);
        assert_eq!(format.repo, None);
    }

    #[test]
    fn parse_issue_keeps_known_format_subcommand() {
        let cmd = IssueCommand::from_args(&["repo-to-md", "issue"], &["format", "42"]).unwrap();
        let IssueSubcommand::Format(format) = cmd.command;

        assert_eq!(format.issue_number, 42);
    }
}
