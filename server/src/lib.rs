//! PlanSpec Server Library
//!
//! This crate provides the PlanSpec API server as a library, allowing it to be
//! embedded in other applications (like the CLI's `serve` command).

use anyhow::Result;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::trace::TraceLayer;

mod controllers;
mod routes;
mod storage;
mod watch;

#[cfg(test)]
mod tests;

pub use controllers::PlanResolver;
pub use storage::Store;
pub use watch::WatchBroadcaster;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub broadcaster: WatchBroadcaster,
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub db_path: String,
    pub reconcile_interval_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            db_path: "planspec.db".to_string(),
            reconcile_interval_secs: 5,
        }
    }
}

impl ServerConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("PLANSPEC_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            db_path: std::env::var("PLANSPEC_DB").unwrap_or_else(|_| "planspec.db".to_string()),
            reconcile_interval_secs: std::env::var("PLANSPEC_RECONCILE_INTERVAL")
                .ok()
                .and_then(|i| i.parse().ok())
                .unwrap_or(5),
        }
    }
}

/// Build the application router with the given state
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .nest("/apis/planspec.io/v1alpha1", routes::api_routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Run the PlanSpec server with the given configuration
pub async fn run_server(config: ServerConfig) -> Result<()> {
    // Initialize storage
    let store = Store::new(&config.db_path).await?;

    // Initialize watch broadcaster
    let broadcaster = WatchBroadcaster::new();

    let state = AppState {
        store: store.clone(),
        broadcaster: broadcaster.clone(),
    };

    // Start the plan resolution controller as a background task
    let resolver = PlanResolver::new(store, broadcaster);
    let interval = Duration::from_secs(config.reconcile_interval_secs);
    tokio::spawn(async move {
        loop {
            if let Err(e) = resolver.reconcile_all().await {
                tracing::warn!(error = %e, "Plan resolution reconcile failed");
            }
            tokio::time::sleep(interval).await;
        }
    });

    // Build router
    let app = build_router(state);

    // Start server
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    tracing::info!("PlanSpec server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
