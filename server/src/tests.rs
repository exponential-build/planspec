//! API endpoint tests for the PlanSpec server.
//!
//! These tests verify all API endpoints work correctly.

#[cfg(test)]
mod api_tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use crate::{routes, storage::Store, watch::WatchBroadcaster, AppState};

    /// Create a test app with an in-memory database
    async fn test_app() -> axum::Router {
        let store = Store::new(":memory:").await.unwrap();
        let broadcaster = WatchBroadcaster::new();
        let state = AppState { store, broadcaster };

        axum::Router::new()
            .nest("/apis/planspec.io/v1alpha1", routes::api_routes())
            .with_state(state)
    }

    /// Helper to make a request and get the response
    async fn request(
        app: axum::Router,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let body = body
            .map(|v| Body::from(serde_json::to_string(&v).unwrap()))
            .unwrap_or(Body::empty());

        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(body)
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        let status = response.status();

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&body_bytes).unwrap_or(json!({}));

        (status, body)
    }

    fn test_goal() -> Value {
        json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Goal",
            "metadata": {
                "name": "test-goal",
                "namespace": "default"
            },
            "spec": {
                "description": "Test goal",
                "acceptanceCriteria": [
                    {"description": "Test criterion"}
                ],
                "priority": 100
            }
        })
    }

    fn test_plan() -> Value {
        json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Plan",
            "metadata": {
                "name": "test-plan",
                "namespace": "default"
            },
            "spec": {
                "description": "Test plan",
                "graph": {
                    "nodes": [
                        {"id": "step-1", "kind": "Task", "description": "Step 1"}
                    ],
                    "edges": []
                }
            }
        })
    }

    fn test_capability() -> Value {
        json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Capability",
            "metadata": {
                "name": "test-capability",
                "namespace": "default"
            },
            "spec": {
                "description": "Test capability"
            }
        })
    }

    // ==================== Health Check ====================

    #[tokio::test]
    async fn test_health_endpoint() {
        let store = Store::new(":memory:").await.unwrap();
        let broadcaster = WatchBroadcaster::new();
        let state = AppState { store, broadcaster };

        let app = axum::Router::new()
            .route("/healthz", axum::routing::get(|| async { "ok" }))
            .with_state(state);

        let (status, _) = request(app, "GET", "/healthz", None).await;
        assert_eq!(status, StatusCode::OK);
    }

    // ==================== Namespace Endpoints ====================

    #[tokio::test]
    async fn test_default_namespace_exists() {
        let app = test_app().await;
        let (status, body) =
            request(app, "GET", "/apis/planspec.io/v1alpha1/namespaces", None).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["kind"], "NamespaceList");

        // New databases should always have the "default" namespace
        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["metadata"]["name"], "default");
    }

    #[tokio::test]
    async fn test_create_namespace() {
        let app = test_app().await;

        let ns = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Namespace",
            "metadata": {
                "name": "test-ns"
            }
        });

        let (status, body) = request(
            app,
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces",
            Some(ns),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["kind"], "Namespace");
        assert_eq!(body["metadata"]["name"], "test-ns");
        assert!(body["metadata"]["uid"].as_str().is_some());
        assert!(body["metadata"]["resourceVersion"].as_str().is_some());
        assert!(body["metadata"]["creationTimestamp"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_create_namespace_conflict() {
        let app = test_app().await;

        let ns = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Namespace",
            "metadata": {
                "name": "test-ns"
            }
        });

        // Create first
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces",
            Some(ns.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Try to create again
        let (status, body) = request(
            app,
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces",
            Some(ns),
        )
        .await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["reason"], "AlreadyExists");
    }

    #[tokio::test]
    async fn test_get_namespace() {
        let app = test_app().await;

        let ns = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Namespace",
            "metadata": {
                "name": "test-ns"
            }
        });

        // Create first
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces",
            Some(ns),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Get it
        let (status, body) = request(
            app,
            "GET",
            "/apis/planspec.io/v1alpha1/namespaces/test-ns",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["kind"], "Namespace");
        assert_eq!(body["metadata"]["name"], "test-ns");
    }

    #[tokio::test]
    async fn test_get_namespace_not_found() {
        let app = test_app().await;

        let (status, body) = request(
            app,
            "GET",
            "/apis/planspec.io/v1alpha1/namespaces/nonexistent",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["reason"], "NotFound");
    }

    #[tokio::test]
    async fn test_delete_namespace() {
        let app = test_app().await;

        let ns = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Namespace",
            "metadata": {
                "name": "test-ns"
            }
        });

        // Create first
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces",
            Some(ns),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Delete it
        let (status, body) = request(
            app.clone(),
            "DELETE",
            "/apis/planspec.io/v1alpha1/namespaces/test-ns",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "Success");

        // Verify it's gone
        let (status, _) = request(
            app,
            "GET",
            "/apis/planspec.io/v1alpha1/namespaces/test-ns",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_namespace_not_found() {
        let app = test_app().await;

        let (status, body) = request(
            app,
            "DELETE",
            "/apis/planspec.io/v1alpha1/namespaces/nonexistent",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["reason"], "NotFound");
    }

    #[tokio::test]
    async fn test_delete_namespace_cascades() {
        let app = test_app().await;

        // Create a namespace
        let ns = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Namespace",
            "metadata": {
                "name": "cascade-test"
            }
        });
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces",
            Some(ns),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Create a goal in that namespace
        let mut goal = test_goal();
        goal["metadata"]["namespace"] = json!("cascade-test");
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/cascade-test/goals",
            Some(goal),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Verify goal exists
        let (status, _) = request(
            app.clone(),
            "GET",
            "/apis/planspec.io/v1alpha1/namespaces/cascade-test/goals/test-goal",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        // Delete namespace
        let (status, body) = request(
            app.clone(),
            "DELETE",
            "/apis/planspec.io/v1alpha1/namespaces/cascade-test",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "Success");

        // Verify goal is gone
        let (status, _) = request(
            app,
            "GET",
            "/apis/planspec.io/v1alpha1/namespaces/cascade-test/goals/test-goal",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_namespaces_shows_empty() {
        let app = test_app().await;

        // Create an empty namespace
        let ns = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Namespace",
            "metadata": {
                "name": "empty-ns"
            }
        });
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces",
            Some(ns),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // List namespaces - should show both default and empty namespace
        let (status, body) =
            request(app, "GET", "/apis/planspec.io/v1alpha1/namespaces", None).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["kind"], "NamespaceList");

        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 2); // default + empty-ns

        let ns_names: Vec<&str> = items
            .iter()
            .filter_map(|item| item["metadata"]["name"].as_str())
            .collect();
        assert!(ns_names.contains(&"default"));
        assert!(ns_names.contains(&"empty-ns"));
    }

    #[tokio::test]
    async fn test_auto_create_namespace_on_resource_create() {
        let app = test_app().await;

        // Create a resource in a namespace that doesn't exist yet
        let mut goal = test_goal();
        goal["metadata"]["namespace"] = json!("auto-created");

        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/auto-created/goals",
            Some(goal),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Verify the namespace was auto-created
        let (status, body) = request(
            app,
            "GET",
            "/apis/planspec.io/v1alpha1/namespaces/auto-created",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["kind"], "Namespace");
        assert_eq!(body["metadata"]["name"], "auto-created");
    }

    #[tokio::test]
    async fn test_auto_create_namespace_on_apply() {
        let app = test_app().await;

        // Apply a resource to a namespace that doesn't exist yet
        let (status, body) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/apply-auto-ns/apply",
            Some(json!([test_goal()])),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["applied"][0]["action"], "created");

        // Verify the namespace was auto-created and appears in the list
        let (status, body) = request(
            app.clone(),
            "GET",
            "/apis/planspec.io/v1alpha1/namespaces",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let items = body["items"].as_array().unwrap();
        let ns_names: Vec<&str> = items
            .iter()
            .filter_map(|item| item["metadata"]["name"].as_str())
            .collect();
        assert!(
            ns_names.contains(&"apply-auto-ns"),
            "Namespace should appear in list after apply: {:?}",
            ns_names
        );

        // Also verify we can get the specific namespace
        let (status, body) = request(
            app,
            "GET",
            "/apis/planspec.io/v1alpha1/namespaces/apply-auto-ns",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["kind"], "Namespace");
        assert_eq!(body["metadata"]["name"], "apply-auto-ns");
    }

    // ==================== Goal Endpoints ====================

    #[tokio::test]
    async fn test_create_goal() {
        let app = test_app().await;

        let (status, body) = request(
            app,
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals",
            Some(test_goal()),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["kind"], "Goal");
        assert_eq!(body["metadata"]["name"], "test-goal");
        assert!(body["metadata"]["uid"].as_str().is_some());
        assert!(body["metadata"]["resourceVersion"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_create_goal_conflict() {
        let app = test_app().await;

        // Create first
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals",
            Some(test_goal()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Try to create again
        let (status, body) = request(
            app,
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals",
            Some(test_goal()),
        )
        .await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["reason"], "AlreadyExists");
    }

    #[tokio::test]
    async fn test_get_goal() {
        let app = test_app().await;

        // Create first
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals",
            Some(test_goal()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Get it
        let (status, body) = request(
            app,
            "GET",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals/test-goal",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["kind"], "Goal");
        assert_eq!(body["metadata"]["name"], "test-goal");
    }

    #[tokio::test]
    async fn test_get_goal_not_found() {
        let app = test_app().await;

        let (status, body) = request(
            app,
            "GET",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals/nonexistent",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["reason"], "NotFound");
    }

    #[tokio::test]
    async fn test_list_goals() {
        let app = test_app().await;

        // Create a goal
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals",
            Some(test_goal()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // List goals
        let (status, body) = request(
            app,
            "GET",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["kind"], "GoalList");

        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["metadata"]["name"], "test-goal");
    }

    #[tokio::test]
    async fn test_list_goals_all_namespaces() {
        let app = test_app().await;

        // Create goals in different namespaces
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals",
            Some(test_goal()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        let mut goal2 = test_goal();
        goal2["metadata"]["name"] = json!("test-goal-2");
        goal2["metadata"]["namespace"] = json!("other");
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/other/goals",
            Some(goal2),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // List all goals
        let (status, body) = request(app, "GET", "/apis/planspec.io/v1alpha1/goals", None).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["kind"], "GoalList");

        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_update_goal() {
        let app = test_app().await;

        // Create first
        let (status, created) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals",
            Some(test_goal()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Update it
        let mut updated = test_goal();
        updated["spec"]["description"] = json!("Updated description");
        updated["metadata"]["resourceVersion"] = created["metadata"]["resourceVersion"].clone();

        let (status, body) = request(
            app,
            "PUT",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals/test-goal",
            Some(updated),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["spec"]["description"], "Updated description");
    }

    #[tokio::test]
    async fn test_delete_goal() {
        let app = test_app().await;

        // Create first
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals",
            Some(test_goal()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Delete it
        let (status, body) = request(
            app.clone(),
            "DELETE",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals/test-goal",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "Success");

        // Verify it's gone
        let (status, _) = request(
            app,
            "GET",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals/test-goal",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_goal_not_found() {
        let app = test_app().await;

        let (status, body) = request(
            app,
            "DELETE",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals/nonexistent",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["reason"], "NotFound");
    }

    // ==================== Plan Endpoints ====================

    #[tokio::test]
    async fn test_create_plan() {
        let app = test_app().await;

        let (status, body) = request(
            app,
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/plans",
            Some(test_plan()),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["kind"], "Plan");
        assert_eq!(body["metadata"]["name"], "test-plan");
        // Plan should have auto-populated status
        assert_eq!(body["status"]["phase"], "Ready");
        assert_eq!(body["status"]["nodeCount"], 1);
    }

    #[tokio::test]
    async fn test_get_plan_graph() {
        let app = test_app().await;

        // Create first
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/plans",
            Some(test_plan()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Get graph
        let (status, body) = request(
            app,
            "GET",
            "/apis/planspec.io/v1alpha1/namespaces/default/plans/test-plan/graph",
            None,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["nodes"].as_array().is_some());
        assert_eq!(body["nodes"].as_array().unwrap().len(), 1);
    }

    // ==================== Capability Endpoints ====================

    #[tokio::test]
    async fn test_create_capability() {
        let app = test_app().await;

        let (status, body) = request(
            app,
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/capabilities",
            Some(test_capability()),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["kind"], "Capability");
        assert_eq!(body["metadata"]["name"], "test-capability");
    }

    // ==================== Status Subresource ====================

    #[tokio::test]
    async fn test_update_status() {
        let app = test_app().await;

        // Create a goal first
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals",
            Some(test_goal()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Update status
        let status_update = json!({
            "status": {
                "phase": "Ready",
                "conditions": [{
                    "type": "TestCondition",
                    "status": "True",
                    "reason": "TestReason"
                }]
            }
        });

        let (status, body) = request(
            app,
            "PUT",
            "/apis/planspec.io/v1alpha1/namespaces/default/goals/test-goal/status",
            Some(status_update),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"]["phase"], "Ready");
        assert_eq!(body["status"]["conditions"][0]["type"], "TestCondition");
    }

    // ==================== Validation Endpoint ====================

    #[tokio::test]
    async fn test_validate_valid_resource() {
        let app = test_app().await;

        // Validate endpoint expects an array of resources
        let (status, body) = request(
            app,
            "POST",
            "/apis/planspec.io/v1alpha1/validate",
            Some(json!([test_goal()])),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["valid"], true);
        assert_eq!(body["results"][0]["valid"], true);
    }

    #[tokio::test]
    async fn test_validate_invalid_resource() {
        let app = test_app().await;

        let invalid = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Goal",
            "metadata": {
                "name": "Invalid_Name"  // Invalid: contains underscore
            },
            "spec": {}
        });

        // Validate endpoint expects an array and always returns 200
        // with valid: false in results for invalid resources
        let (status, body) = request(
            app,
            "POST",
            "/apis/planspec.io/v1alpha1/validate",
            Some(json!([invalid])),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["valid"], false);
        assert_eq!(body["results"][0]["valid"], false);
    }

    // ==================== Apply Endpoint ====================

    #[tokio::test]
    async fn test_apply_create() {
        let app = test_app().await;

        // Apply endpoint expects an array of resources
        let (status, body) = request(
            app,
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/apply",
            Some(json!([test_goal()])),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["applied"][0]["action"], "created");
        assert_eq!(body["errors"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_apply_update() {
        let app = test_app().await;

        // Apply first time (create)
        let (status, _) = request(
            app.clone(),
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/apply",
            Some(json!([test_goal()])),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        // Apply second time (update)
        let mut updated = test_goal();
        updated["spec"]["description"] = json!("Updated");

        let (status, body) = request(
            app,
            "POST",
            "/apis/planspec.io/v1alpha1/namespaces/default/apply",
            Some(json!([updated])),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["applied"][0]["action"], "updated");
        assert_eq!(body["errors"].as_array().unwrap().len(), 0);
    }

    // ==================== Error Handling ====================

    #[tokio::test]
    async fn test_invalid_json() {
        let store = Store::new(":memory:").await.unwrap();
        let broadcaster = WatchBroadcaster::new();
        let state = AppState { store, broadcaster };

        let app = axum::Router::new()
            .nest("/apis/planspec.io/v1alpha1", routes::api_routes())
            .with_state(state);

        let req = Request::builder()
            .method("POST")
            .uri("/apis/planspec.io/v1alpha1/namespaces/default/goals")
            .header("content-type", "application/json")
            .body(Body::from("not valid json"))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        // Axum returns 400 Bad Request for JSON parsing failures
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
