use axum::{http::StatusCode, Json};
use planspec_core::Validator;
use serde_json::{json, Value};

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

        match validator.validate_json(resource) {
            Ok(()) => {
                results.push(json!({
                    "index": i,
                    "kind": kind,
                    "name": name,
                    "valid": true
                }));
            }
            Err(errors) => {
                let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
                results.push(json!({
                    "index": i,
                    "kind": kind,
                    "name": name,
                    "valid": false,
                    "errors": error_messages
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
