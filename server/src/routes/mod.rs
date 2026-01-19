mod apply;
mod resources;
mod validate;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        // Validation endpoint (no namespace)
        .route("/validate", post(validate::validate))
        // Namespaced resource routes
        .route(
            "/namespaces/:namespace/:resource",
            get(resources::list).post(resources::create),
        )
        .route(
            "/namespaces/:namespace/:resource/:name",
            get(resources::get_one)
                .put(resources::replace)
                .delete(resources::delete),
        )
        .route(
            "/namespaces/:namespace/:resource/:name/status",
            put(resources::update_status),
        )
        // Apply endpoint
        .route("/namespaces/:namespace/apply", post(apply::apply))
        // Watch endpoint
        .route(
            "/namespaces/:namespace/:resource/watch",
            get(resources::watch),
        )
        // Plan graph view
        .route(
            "/namespaces/:namespace/plans/:name/graph",
            get(resources::get_plan_graph),
        )
        // Cross-namespace listing
        .route("/:resource", get(resources::list_all))
}

use axum::routing::put;
