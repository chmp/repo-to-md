use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use argh::FromArgs;

use crate::diff::SideBySideDiff;
use crate::formatting::{group_comments_by_file, write_comments_as_markdown};
use crate::local::CommentsFile;

/// Format local review comments as markdown for LLM consumption
#[derive(FromArgs)]
#[argh(subcommand, name = "format")]
pub struct FormatCommand {
    /// path to comments JSON file (default: review-comments.json)
    #[argh(positional, default = "PathBuf::from(\"review-comments.json\")")]
    pub comments_file: PathBuf,

    /// output file (default: stdout)
    #[argh(option, short = 'o')]
    pub output: Option<PathBuf>,
}

impl std::default::Default for FormatCommand {
    fn default() -> Self {
        Self {
            comments_file: PathBuf::from("review-comments.json"),
            output: None,
        }
    }
}

impl FormatCommand {
    pub fn run(self) -> Result<()> {
        let comments_file = CommentsFile::from_path(&self.comments_file).context(format!(
            "Failed to open comments file: {path}. Please run `repo-to-md local review` to create one",
            path = self.comments_file.display(),
        ))?;

        if comments_file.comments.is_empty() {
            eprintln!("No comments to export.");
            return Ok(());
        }

        let diff = SideBySideDiff::parse(&comments_file.raw_diff);

        // Populate diff_hunk for comments that have line numbers
        let mut comments = comments_file.comments;
        for comment in &mut comments {
            if comment.path == "__global__" || comment.line.is_none() {
                continue;
            }

            if let Some(line) = comment.line
                && let Some(hunk) = diff.find_hunk(&comment.path, line)
            {
                comment.diff_hunk = hunk.to_unified();
            }
        }

        let grouped_comments = group_comments_by_file(comments);

        match &self.output {
            Some(path) => {
                let file = File::create(path).context(format!(
                    "Failed to create output file: {path}",
                    path = path.display(),
                ))?;
                let mut writer = BufWriter::new(file);
                write_comments_as_markdown(&mut writer, grouped_comments)?;
                writer.flush()?;
                eprintln!("Exported comments to {path}", path = path.display());
            }
            None => {
                let mut stdout = io::stdout();
                write_comments_as_markdown(&mut stdout, grouped_comments)?;
            }
        }

        Ok(())
    }
}
