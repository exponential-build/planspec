use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::{validate::Validator, AppState};

/// Apply multiple resources atomically within a namespace
pub async fn apply(
    State(state): State<AppState>,
    Path(namespace): Path<String>,
    Json(resources): Json<Vec<Value>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if resources.is_empty() {
        return Ok(Json(json!({
            "applied": [],
            "errors": []
        })));
    }

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

    // First pass: validate all resources
    let mut validation_errors = Vec::new();
    for (i, resource) in resources.iter().enumerate() {
        let kind = resource
            .get("kind")
            .and_then(|k| k.as_str())
            .unwrap_or("Unknown");
        let name = resource
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("<unnamed>");

        match validator.validate(resource) {
            Ok(errors) if !errors.is_empty() => {
                validation_errors.push(json!({
                    "index": i,
                    "resource": format!("{}/{}", kind, name),
                    "reason": "Invalid",
                    "message": errors.join("; ")
                }));
            }
            Err(e) => {
                validation_errors.push(json!({
                    "index": i,
                    "resource": format!("{}/{}", kind, name),
                    "reason": "ValidationError",
                    "message": e.to_string()
                }));
            }
            _ => {}
        }
    }

    if !validation_errors.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "kind": "Status",
                "status": "Failure",
                "message": "Validation failed",
                "reason": "Invalid",
                "details": {
                    "causes": validation_errors
                },
                "code": 400
            })),
        ));
    }

    // Second pass: apply all resources
    let mut applied = Vec::new();
    let mut errors = Vec::new();

    for mut resource in resources {
        let kind = resource
            .get("kind")
            .and_then(|k| k.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let name = resource
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();

        // Ensure namespace is set
        if let Some(metadata) = resource.get_mut("metadata").and_then(|m| m.as_object_mut()) {
            metadata.insert("namespace".to_string(), Value::String(namespace.clone()));
        }

        // Check if exists
        let existing = state.store.get(&namespace, &kind, &name).await;

        match existing {
            Ok(Some(_)) => {
                // Update existing
                match state
                    .store
                    .replace(&namespace, &kind, &name, resource, None)
                    .await
                {
                    Ok((_stored, event)) => {
                        state.broadcaster.send(event);
                        applied.push(json!({
                            "kind": kind,
                            "name": name,
                            "namespace": namespace,
                            "action": "updated"
                        }));
                    }
                    Err(e) => {
                        errors.push(json!({
                            "resource": format!("{}/{}", kind, name),
                            "message": e.to_string()
                        }));
                    }
                }
            }
            Ok(None) => {
                // Create new
                match state.store.create(&namespace, &kind, &name, resource).await {
                    Ok((_stored, event)) => {
                        state.broadcaster.send(event);
                        applied.push(json!({
                            "kind": kind,
                            "name": name,
                            "namespace": namespace,
                            "action": "created"
                        }));
                    }
                    Err(e) => {
                        errors.push(json!({
                            "resource": format!("{}/{}", kind, name),
                            "message": e.to_string()
                        }));
                    }
                }
            }
            Err(e) => {
                errors.push(json!({
                    "resource": format!("{}/{}", kind, name),
                    "message": e.to_string()
                }));
            }
        }
    }

    if !errors.is_empty() {
        // Partial failure
        Ok(Json(json!({
            "applied": applied,
            "errors": errors
        })))
    } else {
        Ok(Json(json!({
            "applied": applied,
            "errors": []
        })))
    }
}
