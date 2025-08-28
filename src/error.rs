use axum::{http::StatusCode, response::{IntoResponse, Html}};
use thiserror::Error;

pub type Result<T, E = AppError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("generation failed: {0}")]
    Generation(String),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match &self {
            AppError::Generation(m) => (StatusCode::BAD_GATEWAY, m.clone()),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into()),
        };
        let safe = html_escape::encode_text(&msg);
        (status, Html(format!(r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Error</title></head>
<body><h1>Oops!</h1><p>Failed to generate HTML.</p>
<pre style="white-space:pre-wrap">{safe}</pre></body></html>"#))).into_response()
    }
}
