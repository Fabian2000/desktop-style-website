use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use tokio::fs;

use crate::state::AppPaths;

/// SPA fallback: serve index.html for any non-static path.
pub async fn spa_fallback(State(paths): State<AppPaths>) -> Response {
    match fs::read_to_string(&paths.index_path).await {
        Ok(content) => Html(content).into_response(),
        Err(err) => {
            // Return 500 with a clear message instead of panicking.
            let msg = format!(
                "index.html not found at: {}\nerror: {}",
                paths.index_path.display(),
                err
            );
            (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
        }
    }
}
