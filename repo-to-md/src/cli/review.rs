use std::io::Write;

use anyhow::Result;
use argh::{EarlyExit, FromArgs};

use crate::{
    cli::{review_format::ReviewFormatCommand, review_local::ReviewLocalCommand},
    client::{
        FetchReviewCommentsClient, GetCurrentUserClient, ListPullRequestsClient, ListReviewsClient,
    },
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
            ReviewSubcommand::Format(cmd) => cmd.check_requirements(),
            ReviewSubcommand::Local(cmd) => cmd.check_requirements(),
        }
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

    #[test]
    fn parse_review_local_accepts_an_optional_end_ref() {
        let cmd =
            ReviewCommand::from_args(&["repo-to-md", "review"], &["local", "main", "feature"])
                .unwrap();
        let ReviewSubcommand::Local(local) = cmd.command else {
            panic!("expected review local command");
        };

        assert_eq!(
            local.refs,
            vec![String::from("main"), String::from("feature")]
        );
    }

    #[test]
    fn parse_review_local_rejects_removed_force_switch() {
        let result = ReviewCommand::from_args(&["repo-to-md", "review"], &["local", "--force"]);

        assert!(result.is_err());
    }

    #[test]
    fn parse_review_format_local_switch() {
        let cmd =
            ReviewCommand::from_args(&["repo-to-md", "review"], &["format", "--local"]).unwrap();
        let ReviewSubcommand::Format(format) = cmd.command else {
            panic!("expected review format command");
        };

        assert!(format.local);
        assert_eq!(format.pr_or_file, None);
    }

    #[test]
    fn parse_review_local_output_override() {
        let cmd = ReviewCommand::from_args(
            &["repo-to-md", "review"],
            &["local", "-o", "custom-session.json"],
        )
        .unwrap();
        let ReviewSubcommand::Local(local) = cmd.command else {
            panic!("expected review local command");
        };

        assert_eq!(local.output, Some(PathBuf::from("custom-session.json")));
    }

    #[test]
    fn parse_review_local_diff_file_mode() {
        let cmd = ReviewCommand::from_args(
            &["repo-to-md", "review"],
            &["local", "--diff", "saved.patch", "--no-open"],
        )
        .unwrap();
        let ReviewSubcommand::Local(local) = cmd.command else {
            panic!("expected review local command");
        };

        assert!(local.refs.is_empty());
        assert_eq!(local.diff, Some(PathBuf::from("saved.patch")));
        assert!(local.no_open);
    }

    #[test]
    fn parse_review_local_single_commit_mode() {
        let cmd = ReviewCommand::from_args(
            &["repo-to-md", "review"],
            &["local", "--commit", "HEAD~2", "--port", "9000"],
        )
        .unwrap();
        let ReviewSubcommand::Local(local) = cmd.command else {
            panic!("expected review local command");
        };

        assert!(local.refs.is_empty());
        assert_eq!(local.commit.as_deref(), Some("HEAD~2"));
        assert_eq!(local.port, Some(9000));
    }

    #[test]
    fn parse_review_local_diff_and_commit_options_together() {
        let cmd = ReviewCommand::from_args(
            &["repo-to-md", "review"],
            &["local", "--diff", "saved.patch", "--commit", "HEAD"],
        )
        .unwrap();
        let ReviewSubcommand::Local(local) = cmd.command else {
            panic!("expected review local command");
        };

        assert!(local.refs.is_empty());
        assert_eq!(local.diff, Some(PathBuf::from("saved.patch")));
        assert_eq!(local.commit.as_deref(), Some("HEAD"));
    }

    #[test]
    fn parse_review_local_diff_option_with_positional_refs() {
        let cmd = ReviewCommand::from_args(
            &["repo-to-md", "review"],
            &["local", "main", "--diff", "saved.patch"],
        )
        .unwrap();
        let ReviewSubcommand::Local(local) = cmd.command else {
            panic!("expected review local command");
        };

        assert_eq!(local.refs, vec![String::from("main")]);
        assert_eq!(local.diff, Some(PathBuf::from("saved.patch")));
    }
}
