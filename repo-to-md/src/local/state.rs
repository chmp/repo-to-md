use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use super::refspec::RefSpec;
use crate::client::Comment;
use crate::diff::SideBySideDiff;

/// JSON file schema for persisted review session (diff + comments)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentsFile {
    pub version: u32,
    pub start_ref: String,
    /// Resolved SHA for start_ref (for validation)
    #[serde(default)]
    pub start_sha: String,
    /// End ref (default to empty string for old files)
    #[serde(default)]
    pub end_ref: String,
    /// Resolved SHA for end_ref (for validation)
    #[serde(default)]
    pub end_sha: String,
    /// Raw unified diff captured at session start
    #[serde(default)]
    pub raw_diff: String,
    pub comments: Vec<Comment>,
    /// Files marked as viewed by the user
    #[serde(default)]
    pub viewed_files: Vec<String>,
}

impl CommentsFile {
    pub fn from_path(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        Ok(serde_json::from_reader(file)?)
    }
}

/// Application state shared across all handlers
pub struct AppState {
    pub refspec: RefSpec,
    pub comments_file_path: PathBuf,
    pub comments: RwLock<Vec<Comment>>,
    pub viewed_files: RwLock<Vec<String>>,
    /// Pre-loaded and highlighted diff
    pub diff: SideBySideDiff,
    /// Raw diff text for persistence
    raw_diff: String,
    file_mtime: RwLock<Option<SystemTime>>,
}

impl AppState {
    pub fn new(
        refspec: RefSpec,
        comments_file_path: PathBuf,
        diff: SideBySideDiff,
        raw_diff: String,
    ) -> Result<Arc<Self>> {
        let (comments, viewed_files, mtime) = Self::load_comments(&comments_file_path)?;

        Ok(Arc::new(AppState {
            refspec,
            comments_file_path,
            comments: RwLock::new(comments),
            viewed_files: RwLock::new(viewed_files),
            diff,
            raw_diff,
            file_mtime: RwLock::new(mtime),
        }))
    }

    fn load_comments(path: &PathBuf) -> Result<(Vec<Comment>, Vec<String>, Option<SystemTime>)> {
        if !path.exists() {
            return Ok((Vec::new(), Vec::new(), None));
        }

        // Get file modification time
        let mtime = fs::metadata(path).ok().and_then(|m| m.modified().ok());

        let content = fs::read_to_string(path).context("Failed to read comments file")?;

        let file: CommentsFile =
            serde_json::from_str(&content).context("Failed to parse comments file")?;

        Ok((file.comments, file.viewed_files, mtime))
    }

    pub fn save_comments(&self) -> Result<()> {
        // Check for external modifications
        if self.comments_file_path.exists() {
            let current_mtime = fs::metadata(&self.comments_file_path)
                .ok()
                .and_then(|m| m.modified().ok());

            let stored_mtime = self.file_mtime.read().expect("lock poisoned");

            if let Some(current) = current_mtime
                && let Some(stored) = *stored_mtime
                && current != stored
            {
                anyhow::bail!(
                    "Comments file was modified externally. \
                     Please reload or resolve conflicts manually."
                );
            }
        }

        let comments = self.comments.read().expect("lock poisoned");
        let viewed_files = self.viewed_files.read().expect("lock poisoned");
        let file = CommentsFile {
            version: 1,
            start_ref: self.refspec.start_ref.clone(),
            start_sha: self.refspec.start_sha.clone(),
            end_ref: self.refspec.end_ref.clone(),
            end_sha: self.refspec.end_sha.clone(),
            raw_diff: self.raw_diff.clone(),
            comments: comments.clone(),
            viewed_files: viewed_files.clone(),
        };

        atomic_write_json(&self.comments_file_path, &file)?;

        // Update stored mtime
        if let Ok(metadata) = fs::metadata(&self.comments_file_path)
            && let Ok(mtime) = metadata.modified()
        {
            *self.file_mtime.write().expect("lock poisoned") = Some(mtime);
        }

        Ok(())
    }

    pub fn add_comment(&self, comment: Comment) -> Result<Comment> {
        {
            let mut comments = self.comments.write().expect("lock poisoned");
            comments.push(comment.clone());
        }
        self.save_comments()?;
        Ok(comment)
    }

    pub fn update_comment(&self, id: &str, body: String) -> Result<Option<Comment>> {
        let updated = {
            let mut comments = self.comments.write().expect("lock poisoned");
            if let Some(comment) = comments.iter_mut().find(|c| c.id == id) {
                comment.body = body;
                Some(comment.clone())
            } else {
                None
            }
        };

        if updated.is_some() {
            self.save_comments()?;
        }

        Ok(updated)
    }

    pub fn delete_comment(&self, id: &str) -> Result<bool> {
        let deleted = {
            let mut comments = self.comments.write().expect("lock poisoned");
            let initial_len = comments.len();
            comments.retain(|c| c.id != id);
            comments.len() < initial_len
        };

        if deleted {
            self.save_comments()?;
        }

        Ok(deleted)
    }

    pub fn get_comments(&self) -> Vec<Comment> {
        self.comments.read().expect("lock poisoned").clone()
    }

    pub fn get_viewed_files(&self) -> Vec<String> {
        self.viewed_files.read().expect("lock poisoned").clone()
    }

    pub fn mark_file_viewed(&self, path: String) -> Result<bool> {
        let added = {
            let mut viewed = self.viewed_files.write().expect("lock poisoned");
            if !viewed.contains(&path) {
                viewed.push(path);
                true
            } else {
                false
            }
        };

        if added {
            self.save_comments()?;
        }

        Ok(added)
    }

    pub fn mark_file_unviewed(&self, path: &str) -> Result<bool> {
        let removed = {
            let mut viewed = self.viewed_files.write().expect("lock poisoned");
            let initial_len = viewed.len();
            viewed.retain(|p| p != path);
            viewed.len() < initial_len
        };

        if removed {
            self.save_comments()?;
        }

        Ok(removed)
    }
}

fn atomic_write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    // Write to temp file in same directory (for atomic rename)
    let parent = path.parent().unwrap_or(Path::new("."));
    let temp_file = NamedTempFile::new_in(parent).context("Failed to create temp file for save")?;

    // Serialize directly to file
    let writer = BufWriter::new(&temp_file);
    serde_json::to_writer_pretty(writer, value).context("Failed to serialize comments")?;

    // Atomic rename
    temp_file
        .persist(path)
        .context("Failed to save comments file")?;

    Ok(())
}
