use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use super::state::AppState;
use crate::client::{Comment, User};

/// Response for GET /api/v1/session - combined endpoint for all read-only data
#[derive(Serialize)]
pub struct SessionResponse {
    pub start_ref: String,
    pub end_ref: String,
    pub files: Vec<crate::diff::FileDiff>,
    pub comments: Vec<Comment>,
    pub viewed_files: Vec<String>,
}

/// GET /api/v1/session - Get all session data (diff, comments, viewed files)
pub async fn get_session(State(state): State<Arc<AppState>>) -> Json<SessionResponse> {
    Json(SessionResponse {
        start_ref: state.refspec.start_ref.clone(),
        end_ref: state.refspec.end_ref.clone(),
        files: state.diff.files.clone(),
        comments: state.get_comments(),
        viewed_files: state.get_viewed_files(),
    })
}

/// Response for comment endpoints
#[derive(Serialize)]
pub struct CommentResponse {
    pub comment: Comment,
}

/// Request body for creating a comment
#[derive(Deserialize)]
pub struct CreateCommentRequest {
    pub path: String,
    /// Line number for file-specific comments, None for global comments
    pub line: Option<u32>,
    pub body: String,
    pub user: String,
    pub diff_hunk: String,
}

/// POST /api/v1/comments - Create a new comment
pub async fn create_comment(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<CommentResponse>), AppError> {
    let comment = Comment {
        id: uuid::Uuid::new_v4().to_string(),
        path: req.path,
        line: req.line,
        body: req.body,
        user: User { login: req.user },
        diff_hunk: req.diff_hunk,
        is_minimized: false,
    };

    let comment = state
        .add_comment(comment)
        .map_err(|e| AppError::Internal(format!("Failed to save comment: {}", e)))?;

    Ok((StatusCode::CREATED, Json(CommentResponse { comment })))
}

/// Request body for updating a comment
#[derive(Deserialize)]
pub struct UpdateCommentRequest {
    pub body: String,
}

/// PUT /api/v1/comments/{id} - Update an existing comment
pub async fn update_comment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCommentRequest>,
) -> Result<Json<CommentResponse>, AppError> {
    let comment = state
        .update_comment(&id, req.body)
        .map_err(|e| AppError::Internal(format!("Failed to update comment: {}", e)))?;

    match comment {
        Some(comment) => Ok(Json(CommentResponse { comment })),
        None => Err(AppError::NotFound(format!("Comment {} not found", id))),
    }
}

/// DELETE /api/v1/comments/{id} - Delete a comment
pub async fn delete_comment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let deleted = state
        .delete_comment(&id)
        .map_err(|e| AppError::Internal(format!("Failed to delete comment: {}", e)))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("Comment {} not found", id)))
    }
}

/// POST /api/v1/comments/{id}/minimize - Toggle minimized state
pub async fn toggle_minimize_comment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<CommentResponse>, AppError> {
    let comment = state
        .toggle_minimize_comment(&id)
        .map_err(|e| AppError::Internal(format!("Failed to toggle minimize: {}", e)))?;

    match comment {
        Some(comment) => Ok(Json(CommentResponse { comment })),
        None => Err(AppError::NotFound(format!("Comment {} not found", id))),
    }
}

/// Request body for setting path status
#[derive(Deserialize)]
pub struct SetPathStatusRequest {
    pub viewed: bool,
}

/// POST /api/v1/paths/{path} - Set path status (viewed/unviewed)
pub async fn set_path_status(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    Json(body): Json<SetPathStatusRequest>,
) -> Result<StatusCode, AppError> {
    if body.viewed {
        state
            .mark_file_viewed(path)
            .map_err(|e| AppError::Internal(format!("Failed to mark file as viewed: {e}")))?;
    } else {
        state
            .mark_file_unviewed(&path)
            .map_err(|e| AppError::Internal(format!("Failed to mark file as unviewed: {e}")))?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Application error type
pub enum AppError {
    NotFound(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = serde_json::json!({ "error": message });
        (status, Json(body)).into_response()
    }
}
