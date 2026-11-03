use dashmap::DashMap;
use monopoly_server::{create_router, AppState};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let registry = Arc::new(DashMap::new());
    let state = AppState { registry };

    let app = create_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await?;

    info!("Monopoly Mayhem server listening on http://{}", addr);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
