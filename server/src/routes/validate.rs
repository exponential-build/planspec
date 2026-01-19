use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

use crate::validate::Validator;

/// Validate resources without persisting
pub async fn validate(
    Json(resources): Json<Vec<Value>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
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

    let mut results = Vec::new();

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
            Ok(errors) if errors.is_empty() => {
                results.push(json!({
                    "index": i,
                    "kind": kind,
                    "name": name,
                    "valid": true
                }));
            }
            Ok(errors) => {
                results.push(json!({
                    "index": i,
                    "kind": kind,
                    "name": name,
                    "valid": false,
                    "errors": errors
                }));
            }
            Err(e) => {
                results.push(json!({
                    "index": i,
                    "kind": kind,
                    "name": name,
                    "valid": false,
                    "errors": [e.to_string()]
                }));
            }
        }
    }

    let all_valid = results.iter().all(|r| r.get("valid") == Some(&json!(true)));

    Ok(Json(json!({
        "valid": all_valid,
        "results": results
    })))
}
