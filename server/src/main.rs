use anyhow::Result;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod routes;
mod storage;
mod validate;
mod watch;

use storage::Store;
use watch::WatchBroadcaster;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub broadcaster: WatchBroadcaster,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "planspec_server=debug,tower_http=debug".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize storage
    let db_path = std::env::var("PLANSPEC_DB").unwrap_or_else(|_| "planspec.db".to_string());
    let store = Store::new(&db_path).await?;

    // Initialize watch broadcaster
    let broadcaster = WatchBroadcaster::new();

    let state = AppState { store, broadcaster };

    // Build router
    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .nest("/apis/planspec.io/v1alpha1", routes::api_routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("PlanSpec server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
