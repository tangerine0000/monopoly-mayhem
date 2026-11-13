use dashmap::DashMap;
use monopoly_server::{create_router, AppState};
use std::net::{Ipv6Addr, SocketAddr, SocketAddrV6};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let registry = Arc::new(DashMap::new());
    let state = AppState { registry };

    let app = create_router(state);
    let addr_v4 = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener_v4 = TcpListener::bind(addr_v4).await?;
    info!("Monopoly Mayhem server listening on http://{}", addr_v4);

    let addr_v6 = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 8080, 0, 0));
    if let Ok(listener_v6) = TcpListener::bind(addr_v6).await {
        info!("Monopoly Mayhem server also listening on http://{}", addr_v6);
        let app_v6 = app.clone();
        tokio::select! {
            res = axum::serve(listener_v4, app.into_make_service()) => res?,
            res = axum::serve(listener_v6, app_v6.into_make_service()) => res?,
        }
    } else {
        axum::serve(listener_v4, app.into_make_service()).await?;
    }

    Ok(())
}
