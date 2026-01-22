//! Static file serving for embedded web frontend

use axum::{
    body::Body,
    http::{header, Response, StatusCode, Uri},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

/// Embedded web frontend files
/// The path is relative to the crate root (crates/server)
/// Files are embedded at compile time from ../../web/dist
#[derive(RustEmbed)]
#[folder = "../../web/dist"]
pub struct WebAssets;

/// Handler for serving static files
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // If path is empty or doesn't contain a dot (likely a route), serve index.html
    let path = if path.is_empty() || (!path.contains('.') && !path.starts_with("api")) {
        "index.html"
    } else {
        path
    };

    serve_file(path)
}

/// Serve a specific file from the embedded assets
fn serve_file(path: &str) -> Response<Body> {
    match WebAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            // Determine cache control based on file type
            // Service workers and manifests should not be cached long
            let cache_control = if path == "sw.js" || path.ends_with("registerSW.js") {
                "no-cache, no-store, must-revalidate"
            } else if path == "manifest.webmanifest" {
                "public, max-age=0, must-revalidate"
            } else if path.starts_with("assets/") {
                // Hashed assets can be cached forever
                "public, max-age=31536000, immutable"
            } else {
                "public, max-age=3600"
            };

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .header(header::CACHE_CONTROL, cache_control)
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None => {
            // Try to serve index.html for SPA routes
            if !path.contains('.') {
                if let Some(content) = WebAssets::get("index.html") {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "text/html")
                        .body(Body::from(content.data.into_owned()))
                        .unwrap();
                }
            }

            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("Not Found"))
                .unwrap()
        }
    }
}

/// Check if web assets are available (for conditional routing)
pub fn has_web_assets() -> bool {
    WebAssets::get("index.html").is_some()
}
