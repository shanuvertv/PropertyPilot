//! Everything outside `/api`: the built web UI when a `web` directory is present
//! (`https://<server>/` then opens the same app as the desktop and phone builds),
//! otherwise a small landing response so a browser visit is not a bare 404.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, Router};
use tower_http::services::{ServeDir, ServeFile};

async fn landing() -> Response {
    let body = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>PropertyPilot server</title>         <body style=\"font-family:system-ui;margin:3rem auto;max-width:36rem;line-height:1.5\">         <h1>PropertyPilot server {}</h1>         <p>The server is running. Enter this address in the Windows or Android app to connect.</p>         <p>Health: <a href=\"/api/health\">/api/health</a></p></body>",
        crate::VERSION
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        body,
    )
        .into_response()
}

async fn api_not_found() -> Response {
    StatusCode::NOT_FOUND.into_response()
}

/// Static files with an SPA fallback to `index.html`; unknown `/api/*` paths stay 404.
pub fn fallback(web_dir: Option<PathBuf>) -> Router {
    match web_dir {
        Some(dir) => {
            tracing::info!(dir = %dir.display(), "serving the web UI");
            let index = dir.join("index.html");
            let serve = ServeDir::new(&dir).fallback(ServeFile::new(index));
            Router::new()
                .route("/api/{*rest}", any(api_not_found))
                .fallback_service(serve)
        }
        None => Router::new()
            .route("/api/{*rest}", any(api_not_found))
            .fallback(any(|_req: Request<Body>| async { landing().await })),
    }
}
