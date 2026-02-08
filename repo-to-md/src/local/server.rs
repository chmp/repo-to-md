use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    Router,
    extract::{FromRef, State},
    http::StatusCode,
    routing::{delete, get, post, put},
};
use tokio::sync::broadcast;

use super::assets::serve_static;
use super::handlers::{
    create_comment, delete_comment, get_session, set_path_status, update_comment,
};
use super::refspec::RefSpec;
use super::state::AppState;
use crate::diff::SideBySideDiff;

/// Shared state for shutdown signaling
#[derive(Clone)]
struct ServerState {
    app_state: Arc<AppState>,
    shutdown_tx: broadcast::Sender<()>,
}

// Allow handlers to extract Arc<AppState> from ServerState
impl FromRef<ServerState> for Arc<AppState> {
    fn from_ref(state: &ServerState) -> Self {
        state.app_state.clone()
    }
}

/// A bound server ready to serve requests.
///
/// After binding the port and building the router, this struct provides
/// access to the server URL and a method to start serving.
pub struct BoundServer {
    url: String,
    listener: tokio::net::TcpListener,
    app: Router,
    shutdown_tx: broadcast::Sender<()>,
    comments_file: PathBuf,
}

impl BoundServer {
    /// Get the URL where the server is listening.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Start serving requests.
    ///
    /// Blocks until the server shuts down via Ctrl-C or shutdown signal.
    pub async fn serve(self) -> Result<()> {
        // Wait for either Ctrl-C or shutdown signal
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let shutdown_signal = async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    eprintln!("\nReceived Ctrl-C, shutting down...");
                }
                _ = shutdown_rx.recv() => {
                    eprintln!("\nShutdown requested, closing server...");
                }
            }
        };

        axum::serve(self.listener, self.app)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .context("Server error")?;

        // Print the format command hint
        print_format_hint(&self.comments_file);

        Ok(())
    }
}

/// Bind the server to a port and prepare to serve.
///
/// Returns a `BoundServer` that can be used to start serving requests.
/// The port is bound immediately, but the server doesn't start serving
/// until `BoundServer::serve()` is called.
pub async fn bind_server(
    refspec: RefSpec,
    port: u16,
    comments_file: PathBuf,
    diff: SideBySideDiff,
    raw_diff: String,
) -> Result<BoundServer> {
    let app_state = AppState::new(refspec, comments_file.clone(), diff, raw_diff)?;

    // Create shutdown signal channel
    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    let server_state = ServerState {
        app_state,
        shutdown_tx: shutdown_tx.clone(),
    };

    // Build the router
    let app = Router::new()
        // API routes
        .route("/api/v1/session", get(get_session))
        .route("/api/v1/comments", post(create_comment))
        .route("/api/v1/comments/{id}", put(update_comment))
        .route("/api/v1/comments/{id}", delete(delete_comment))
        .route("/api/v1/paths/{*path}", post(set_path_status))
        .route("/api/v1/shutdown", post(shutdown_handler))
        .with_state(server_state)
        // Static assets (fallback to index.html for SPA routing)
        .fallback(serve_static);

    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context(format!("Failed to bind to {}", addr))?;

    let url = format!("http://{}", addr);
    eprintln!("Server running at {}", url);

    Ok(BoundServer {
        url,
        listener,
        app,
        shutdown_tx,
        comments_file,
    })
}

/// POST /api/v1/shutdown - Gracefully shut down the server
async fn shutdown_handler(State(state): State<ServerState>) -> StatusCode {
    // Send shutdown signal (ignore errors if no receivers)
    let _ = state.shutdown_tx.send(());
    StatusCode::OK
}

fn print_format_hint(comments_file: &std::path::Path) {
    eprintln!();
    eprintln!("To format your review comments as markdown, run:");
    eprintln!();
    eprintln!("  repo-to-md local format {}", comments_file.display());
    eprintln!();
}
