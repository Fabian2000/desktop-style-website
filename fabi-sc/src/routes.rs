use axum::Router;
use tower_http::services::ServeDir;

use crate::{handlers, state::AppPaths};

pub fn create_router(paths: AppPaths) -> Router {
    let static_service = ServeDir::new(paths.resources_dir.clone());

    Router::new()
        .nest_service("/resources", static_service)
        .fallback(handlers::spa_fallback)
        .with_state(paths)
}
