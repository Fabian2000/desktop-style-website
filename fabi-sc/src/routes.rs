use axum::Router;
use tower_http::services::ServeDir;

use crate::{handlers, state::AppPaths};

pub fn create_router(paths: AppPaths) -> Router {
    // Serve only from the configured resources directory. Directory traversal outside is blocked
    // by ServeDir, which normalizes paths and restricts to the given root.
    let static_service = ServeDir::new(paths.resources_dir.clone());

    Router::new()
        .nest_service("/resources", static_service)
        // For any non-/resources path (including 404s), serve index.html
        .fallback(handlers::spa_fallback)
        .with_state(paths)
}
