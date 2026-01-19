use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{sse::Event, Sse},
    Json,
};
use futures::stream::Stream;
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use planspec_core::Validator;

use crate::AppState;

/// Populate Plan status with phase, nodeCount, and validation condition
fn populate_plan_status(body: &mut Value, generation: i64) {
    let node_count = body
        .get("spec")
        .and_then(|s| s.get("graph"))
        .and_then(|g| g.get("nodes"))
        .and_then(|n| n.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    let now = chrono::Utc::now().to_rfc3339();

    // Initialize or update status
    if body.get("status").is_none() {
        body["status"] = json!({});
    }
    body["status"]["phase"] = json!("Ready");
    body["status"]["nodeCount"] = json!(node_count);
    body["status"]["observedGeneration"] = json!(generation);

    // Add Valid condition
    body["status"]["conditions"] = json!([{
        "type": "Valid",
        "status": "True",
        "reason": "SchemaValid",
        "message": "Plan passed schema validation",
        "lastTransitionTime": now,
        "observedGeneration": generation
    }]);
}

/// Map resource type from URL to Kind
fn resource_to_kind(resource: &str) -> &str {
    match resource {
        "goals" => "Goal",
        "plans" => "Plan",
        "capabilities" => "Capability",
        "bindings" => "Binding",
        "executions" => "Execution",
        _ => resource,
    }
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(rename = "labelSelector")]
    pub label_selector: Option<String>,
    #[serde(rename = "allNamespaces")]
    pub all_namespaces: Option<bool>,
}

/// List resources in a namespace
pub async fn list(
    State(state): State<AppState>,
    Path((namespace, resource)): Path<(String, String)>,
    Query(_query): Query<ListQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let kind = resource_to_kind(&resource);

    let items = state
        .store
        .list(&namespace, kind)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "kind": "Status",
                    "status": "Failure",
                    "message": e.to_string(),
                    "code": 500
                })),
            )
        })?;

    let objects: Vec<Value> = items.into_iter().map(|item| item.object).collect();

    Ok(Json(json!({
        "apiVersion": "planspec.io/v1alpha1",
        "kind": format!("{}List", kind),
        "items": objects
    })))
}

/// List resources across all namespaces
pub async fn list_all(
    State(state): State<AppState>,
    Path(resource): Path<String>,
    Query(_query): Query<ListQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let kind = resource_to_kind(&resource);

    let items = state
        .store
        .list_all(kind)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "kind": "Status",
                    "status": "Failure",
                    "message": e.to_string(),
                    "code": 500
                })),
            )
        })?;

    let objects: Vec<Value> = items.into_iter().map(|item| item.object).collect();

    Ok(Json(json!({
        "apiVersion": "planspec.io/v1alpha1",
        "kind": format!("{}List", kind),
        "items": objects
    })))
}

/// Get a single resource
pub async fn get_one(
    State(state): State<AppState>,
    Path((namespace, resource, name)): Path<(String, String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let kind = resource_to_kind(&resource);

    let item = state
        .store
        .get(&namespace, kind, &name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "kind": "Status",
                    "status": "Failure",
                    "message": e.to_string(),
                    "code": 500
                })),
            )
        })?;

    match item {
        Some(obj) => Ok(Json(obj.object)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "kind": "Status",
                "status": "Failure",
                "message": format!("{} '{}' not found in namespace '{}'", kind, name, namespace),
                "reason": "NotFound",
                "code": 404
            })),
        )),
    }
}

/// Create a resource
pub async fn create(
    State(state): State<AppState>,
    Path((namespace, resource)): Path<(String, String)>,
    Json(mut body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let kind = resource_to_kind(&resource);

    // Validate
    let validator = Validator::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "kind": "Status",
                "status": "Failure",
                "message": e.to_string(),
                "code": 500
            })),
        )
    })?;

    if let Err(errors) = validator.validate_json(&body) {
        let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "kind": "Status",
                "status": "Failure",
                "message": "Validation failed",
                "reason": "Invalid",
                "details": {
                    "causes": error_messages
                },
                "code": 400
            })),
        ));
    }

    // Ensure namespace in metadata matches path
    if let Some(metadata) = body.get_mut("metadata").and_then(|m| m.as_object_mut()) {
        metadata.insert("namespace".to_string(), Value::String(namespace.clone()));
    }

    // Populate status for Plans (generation 1 for new resources)
    if kind == "Plan" {
        populate_plan_status(&mut body, 1);
    }

    let name = body
        .get("metadata")
        .and_then(|m| m.get("name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "kind": "Status",
                    "status": "Failure",
                    "message": "metadata.name is required",
                    "reason": "Invalid",
                    "code": 400
                })),
            )
        })?
        .to_string();

    // Check if already exists
    if let Some(_) = state.store.get(&namespace, kind, &name).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "kind": "Status",
                "status": "Failure",
                "message": e.to_string(),
                "code": 500
            })),
        )
    })? {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({
                "kind": "Status",
                "status": "Failure",
                "message": format!("{} '{}' already exists", kind, name),
                "reason": "AlreadyExists",
                "code": 409
            })),
        ));
    }

    let (stored, event) = state
        .store
        .create(&namespace, kind, &name, body)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "kind": "Status",
                    "status": "Failure",
                    "message": e.to_string(),
                    "code": 500
                })),
            )
        })?;

    // Broadcast event
    state.broadcaster.send(event);

    Ok((StatusCode::CREATED, Json(stored.object)))
}

/// Replace a resource
pub async fn replace(
    State(state): State<AppState>,
    Path((namespace, resource, name)): Path<(String, String, String)>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let kind = resource_to_kind(&resource);

    // Validate
    let validator = Validator::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "kind": "Status",
                "status": "Failure",
                "message": e.to_string(),
                "code": 500
            })),
        )
    })?;

    if let Err(errors) = validator.validate_json(&body) {
        let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "kind": "Status",
                "status": "Failure",
                "message": "Validation failed",
                "reason": "Invalid",
                "details": {
                    "causes": error_messages
                },
                "code": 400
            })),
        ));
    }

    // Get resourceVersion from request for optimistic concurrency
    let resource_version = body
        .get("metadata")
        .and_then(|m| m.get("resourceVersion"))
        .and_then(|rv| rv.as_str())
        .map(|s| s.to_string());

    // Ensure namespace in metadata matches path
    if let Some(metadata) = body.get_mut("metadata").and_then(|m| m.as_object_mut()) {
        metadata.insert("namespace".to_string(), Value::String(namespace.clone()));
        metadata.insert("name".to_string(), Value::String(name.clone()));
    }

    // Get existing resource to determine generation for status
    let existing_generation = state
        .store
        .get(&namespace, kind, &name)
        .await
        .ok()
        .flatten()
        .map(|obj| obj.generation)
        .unwrap_or(1);

    // Populate status for Plans (generation may increment on spec change)
    if kind == "Plan" {
        // Check if spec changed - if so, generation will be incremented
        let new_generation = existing_generation + 1; // Assume it might change
        populate_plan_status(&mut body, new_generation);
    }

    let (stored, event) = state
        .store
        .replace(&namespace, kind, &name, body, resource_version.as_deref())
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Conflict") {
                (
                    StatusCode::CONFLICT,
                    Json(json!({
                        "kind": "Status",
                        "status": "Failure",
                        "message": msg,
                        "reason": "Conflict",
                        "code": 409
                    })),
                )
            } else if msg.contains("not found") {
                (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "kind": "Status",
                        "status": "Failure",
                        "message": msg,
                        "reason": "NotFound",
                        "code": 404
                    })),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "kind": "Status",
                        "status": "Failure",
                        "message": msg,
                        "code": 500
                    })),
                )
            }
        })?;

    // Broadcast event
    state.broadcaster.send(event);

    Ok(Json(stored.object))
}

/// Update only the status subresource
pub async fn update_status(
    State(state): State<AppState>,
    Path((namespace, resource, name)): Path<(String, String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let kind = resource_to_kind(&resource);

    // Get existing resource
    let existing = state
        .store
        .get(&namespace, kind, &name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "kind": "Status",
                    "status": "Failure",
                    "message": e.to_string(),
                    "code": 500
                })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "kind": "Status",
                    "status": "Failure",
                    "message": format!("{} '{}' not found", kind, name),
                    "reason": "NotFound",
                    "code": 404
                })),
            )
        })?;

    // Merge: keep existing object, replace only status
    let mut updated = existing.object.clone();
    if let Some(new_status) = body.get("status") {
        updated["status"] = new_status.clone();
    }

    let resource_version = body
        .get("metadata")
        .and_then(|m| m.get("resourceVersion"))
        .and_then(|rv| rv.as_str())
        .map(|s| s.to_string());

    let (stored, event) = state
        .store
        .replace(&namespace, kind, &name, updated, resource_version.as_deref())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "kind": "Status",
                    "status": "Failure",
                    "message": e.to_string(),
                    "code": 500
                })),
            )
        })?;

    state.broadcaster.send(event);

    Ok(Json(stored.object))
}

/// Delete a resource
pub async fn delete(
    State(state): State<AppState>,
    Path((namespace, resource, name)): Path<(String, String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let kind = resource_to_kind(&resource);

    let event = state
        .store
        .delete(&namespace, kind, &name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "kind": "Status",
                    "status": "Failure",
                    "message": e.to_string(),
                    "code": 500
                })),
            )
        })?;

    match event {
        Some(ev) => {
            state.broadcaster.send(ev);
            Ok(Json(json!({
                "kind": "Status",
                "status": "Success",
                "message": format!("{} '{}' deleted", kind, name),
                "code": 200
            })))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "kind": "Status",
                "status": "Failure",
                "message": format!("{} '{}' not found", kind, name),
                "reason": "NotFound",
                "code": 404
            })),
        )),
    }
}

/// Watch resources for changes (NDJSON stream)
pub async fn watch(
    State(state): State<AppState>,
    Path((namespace, resource)): Path<(String, String)>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let kind = resource_to_kind(&resource).to_string();
    let namespace = namespace.clone();

    let rx = state.broadcaster.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(move |result| {
        match result {
            Ok(event) => {
                // Filter by namespace and kind
                let obj_ns = event
                    .object
                    .get("metadata")
                    .and_then(|m| m.get("namespace"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                let obj_kind = event
                    .object
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .unwrap_or("");

                if obj_ns == namespace && obj_kind == kind {
                    let data = serde_json::to_string(&event).unwrap_or_default();
                    Some(Ok(Event::default().data(data)))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    });

    Sse::new(stream)
}

/// Get Plan graph (just returns spec.graph)
pub async fn get_plan_graph(
    State(state): State<AppState>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let item = state
        .store
        .get(&namespace, "Plan", &name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "kind": "Status",
                    "status": "Failure",
                    "message": e.to_string(),
                    "code": 500
                })),
            )
        })?;

    match item {
        Some(obj) => {
            let graph = obj
                .object
                .get("spec")
                .and_then(|s| s.get("graph"))
                .cloned()
                .unwrap_or(json!({"nodes": [], "edges": []}));

            Ok(Json(graph))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "kind": "Status",
                "status": "Failure",
                "message": format!("Plan '{}' not found", name),
                "reason": "NotFound",
                "code": 404
            })),
        )),
    }
}
