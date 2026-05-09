use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::formatting::{group_comments_by_file, write_comments_as_markdown};
use crate::local::CommentsFile;
use crate::side_by_side_diff::SideBySideDiff;

/// Format local review comments as markdown for LLM consumption
pub struct LocalFormatCommand {
    pub comments_file: PathBuf,
    pub output: Option<PathBuf>,
}

impl std::default::Default for LocalFormatCommand {
    fn default() -> Self {
        Self {
            comments_file: PathBuf::from("review-comments.json"),
            output: None,
        }
    }
}

impl LocalFormatCommand {
    pub fn run_with_writer(self, writer: &mut impl Write) -> Result<()> {
        let comments_file = CommentsFile::from_path(&self.comments_file).context(format!(
            "Failed to open comments file: {path}. Please run `repo-to-md review local` to create one",
            path = self.comments_file.display(),
        ))?;

        if comments_file.comments.is_empty() {
            eprintln!("No comments to export.");
            return Ok(());
        }

        let diff = SideBySideDiff::parse(&comments_file.raw_diff)?;

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

        // Filter out minimized comments
        let comments: Vec<_> = comments.into_iter().filter(|c| !c.is_minimized).collect();
        if comments.is_empty() {
            eprintln!("No active comments to export (all minimized).");
            return Ok(());
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
                write_comments_as_markdown(writer, grouped_comments)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_format_filters_minimized_comments() {
        use std::io::Write;

        // Create a temp file with comments including minimized ones
        let mut temp_file = NamedTempFile::new().unwrap();
        let comments_data = serde_json::json!({
            "version": 1,
            "start_ref": "HEAD~1",
            "start_sha": "abc123",
            "end_ref": "HEAD",
            "end_sha": "def456",
            "raw_diff": "",
            "comments": [
                {
                    "id": "1",
                    "path": "test.rs",
                    "line": 1,
                    "body": "Active comment",
                    "diff_hunk": "",
                    "user": { "login": "user1" },
                    "is_minimized": false
                },
                {
                    "id": "2",
                    "path": "test.rs",
                    "line": 2,
                    "body": "Minimized comment",
                    "diff_hunk": "",
                    "user": { "login": "user2" },
                    "is_minimized": true
                }
            ],
            "viewed_files": []
        });
        temp_file
            .write_all(serde_json::to_string(&comments_data).unwrap().as_bytes())
            .unwrap();
        temp_file.flush().unwrap();

        let cmd = LocalFormatCommand {
            comments_file: temp_file.path().to_path_buf(),
            output: None,
        };

        let mut output = Vec::new();
        let result = cmd.run_with_writer(&mut output);
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_all_minimized_comments() {
        use std::io::Write;

        // Create a temp file with all minimized comments
        let mut temp_file = NamedTempFile::new().unwrap();
        let comments_data = serde_json::json!({
            "version": 1,
            "start_ref": "HEAD~1",
            "start_sha": "abc123",
            "end_ref": "HEAD",
            "end_sha": "def456",
            "raw_diff": "",
            "comments": [
                {
                    "id": "1",
                    "path": "test.rs",
                    "line": 1,
                    "body": "Minimized comment 1",
                    "diff_hunk": "",
                    "user": { "login": "user1" },
                    "is_minimized": true
                },
                {
                    "id": "2",
                    "path": "test.rs",
                    "line": 2,
                    "body": "Minimized comment 2",
                    "diff_hunk": "",
                    "user": { "login": "user2" },
                    "is_minimized": true
                }
            ],
            "viewed_files": []
        });
        temp_file
            .write_all(serde_json::to_string(&comments_data).unwrap().as_bytes())
            .unwrap();
        temp_file.flush().unwrap();

        let cmd = LocalFormatCommand {
            comments_file: temp_file.path().to_path_buf(),
            output: None,
        };

        let mut output = Vec::new();
        let result = cmd.run_with_writer(&mut output);
        // Should succeed with no output since all comments are minimized
        assert!(result.is_ok());
    }
}
