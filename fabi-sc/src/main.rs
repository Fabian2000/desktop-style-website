// main.rs
// Goal: Axum SPA-friendly server on port 51287 without unwrap/panic, split into modules.

mod routes;
mod handlers;
mod state;

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build file system paths relative to the executable (fallback: current dir)
    let paths = state::AppPaths::from_exe();

    // Build router with state and static service
    let app = routes::create_router(paths);

    // Listen on 0.0.0.0:51287
    let addr: SocketAddr = ([0, 0, 0, 0], 51287).into();
    println!("Server running on http://{addr}");

    // axum 0.7 style serving without unwrap/expect
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
