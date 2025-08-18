mod routes;
mod handlers;
mod state;

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = state::AppPaths::from_exe();

    let app = routes::create_router(paths);

    let addr: SocketAddr = ([0, 0, 0, 0], 51287).into();
    println!("Server running on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
