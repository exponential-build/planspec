use anyhow::Result;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod controllers;
mod routes;
mod storage;
mod watch;

#[cfg(test)]
mod tests;

use controllers::PlanResolver;
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
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "planspec_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize storage
    let db_path = std::env::var("PLANSPEC_DB").unwrap_or_else(|_| "planspec.db".to_string());
    let store = Store::new(&db_path).await?;

    // Initialize watch broadcaster
    let broadcaster = WatchBroadcaster::new();

    let state = AppState {
        store: store.clone(),
        broadcaster: broadcaster.clone(),
    };

    // Start the plan resolution controller as a background task
    let resolver = PlanResolver::new(store, broadcaster);
    tokio::spawn(async move {
        let interval = Duration::from_secs(5);
        loop {
            if let Err(e) = resolver.reconcile_all().await {
                tracing::warn!(error = %e, "Plan resolution reconcile failed");
            }
            tokio::time::sleep(interval).await;
        }
    });

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
