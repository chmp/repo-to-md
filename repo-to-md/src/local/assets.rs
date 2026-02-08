use axum::{
    body::Body,
    http::{Response, StatusCode, Uri, header},
    response::IntoResponse,
};
use rust_embed::Embed;

use crate::language;

#[derive(Embed)]
#[folder = "src/static/"]
pub struct StaticAssets;

pub async fn serve_static(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Default to index.html for root path
    let path = if path.is_empty() { "index.html" } else { path };
    let Some(content) = StaticAssets::get(path) else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .expect("created 404 response");
    };

    let mime = match language::detect_language(path) {
        "javascript" => "application/javascript",
        "html" => "text/html",
        "css" => "text/css",
        _ => "application/octet-stream",
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .body(Body::from(content.data.into_owned()))
        .expect("created static response")
}
