//! Unit tests for planspec-core types and validation.

use crate::*;
use serde_json::json;

mod serialization {
    use super::*;

    #[test]
    fn goal_round_trip() {
        let goal = Goal::new("my-goal", "default")
            .with_description("Test goal")
            .with_criterion("Must pass tests")
            .with_priority(10);

        let json = serde_json::to_string(&goal).unwrap();
        let parsed: Goal = serde_json::from_str(&json).unwrap();
        assert_eq!(goal, parsed);
    }

    #[test]
    fn plan_round_trip() {
        let plan = Plan::new("my-plan", "default")
            .with_description("Test plan")
            .with_series("test")
            .with_version("1");

        let json = serde_json::to_string(&plan).unwrap();
        let parsed: Plan = serde_json::from_str(&json).unwrap();
        assert_eq!(plan, parsed);
    }

    #[test]
    fn capability_round_trip() {
        let cap = Capability::new("code-gen", "default")
            .with_description("Generate code")
            .with_display_name("Code Generation")
            .with_category("development");

        let json = serde_json::to_string(&cap).unwrap();
        let parsed: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, parsed);
    }

    #[test]
    fn binding_round_trip() {
        let binding = Binding::new("my-binding", "default")
            .with_rule(BindingRule::for_capability("code-gen", "agent://claude"));

        let json = serde_json::to_string(&binding).unwrap();
        let parsed: Binding = serde_json::from_str(&json).unwrap();
        assert_eq!(binding, parsed);
    }

    #[test]
    fn execution_round_trip() {
        let exec = Execution::new("my-exec", "default")
            .with_plan_ref("my-plan")
            .with_goal_ref("my-goal");

        let json = serde_json::to_string(&exec).unwrap();
        let parsed: Execution = serde_json::from_str(&json).unwrap();
        assert_eq!(exec, parsed);
    }

    #[test]
    fn resource_enum_round_trip() {
        let goal = Goal::new("my-goal", "default").with_description("Test");
        let resource = Resource::Goal(goal.clone());

        let json = serde_json::to_string(&resource).unwrap();
        let parsed: Resource = serde_json::from_str(&json).unwrap();

        match parsed {
            Resource::Goal(g) => assert_eq!(g, goal),
            _ => panic!("Expected Goal variant"),
        }
    }

    #[test]
    fn camel_case_serialization() {
        let meta = ObjectMeta::new("test", "default")
            .with_label("app.kubernetes.io/name", "test");

        let json = serde_json::to_string(&meta).unwrap();

        // Should use camelCase
        assert!(json.contains("\"ownerReferences\"") || !json.contains("owner"));
        assert!(!json.contains("owner_references"));
    }
}

mod validation {
    use super::*;

    #[test]
    fn valid_goal_passes() {
        let validator = Validator::new().unwrap();
        let goal = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Goal",
            "metadata": {
                "name": "test-goal",
                "namespace": "default"
            },
            "spec": {
                "description": "A test goal"
            }
        });

        assert!(validator.validate_json(&goal).is_ok());
    }

    #[test]
    fn missing_kind_fails() {
        let validator = Validator::new().unwrap();
        let invalid = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "metadata": {
                "name": "test",
                "namespace": "default"
            },
            "spec": {}
        });

        let result = validator.validate_json(&invalid);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(e, ValidationError::MissingKind)));
    }

    #[test]
    fn unknown_kind_fails() {
        let validator = Validator::new().unwrap();
        let invalid = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "UnknownThing",
            "metadata": {
                "name": "test",
                "namespace": "default"
            },
            "spec": {}
        });

        let result = validator.validate_json(&invalid);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(e, ValidationError::UnknownKind(_))));
    }

    #[test]
    fn invalid_name_format_fails() {
        let validator = Validator::new().unwrap();
        let invalid = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Goal",
            "metadata": {
                "name": "Invalid_Name",  // underscores not allowed
                "namespace": "default"
            },
            "spec": {
                "description": "Test"
            }
        });

        let result = validator.validate_json(&invalid);
        assert!(result.is_err());
    }

    #[test]
    fn valid_plan_passes() {
        let validator = Validator::new().unwrap();
        let plan = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Plan",
            "metadata": {
                "name": "test-plan",
                "namespace": "default"
            },
            "spec": {
                "description": "A test plan",
                "graph": {
                    "nodes": [
                        {
                            "id": "task-1",
                            "kind": "Task",
                            "description": "First task"
                        },
                        {
                            "id": "task-2",
                            "kind": "Task",
                            "description": "Second task"
                        }
                    ],
                    "edges": [
                        { "from": "task-1", "to": "task-2" }
                    ]
                }
            }
        });

        assert!(validator.validate_json(&plan).is_ok());
    }

    #[test]
    fn cyclic_plan_fails() {
        let validator = Validator::new().unwrap();
        let cyclic_plan = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Plan",
            "metadata": {
                "name": "cyclic-plan",
                "namespace": "default"
            },
            "spec": {
                "description": "A cyclic plan",
                "graph": {
                    "nodes": [
                        { "id": "a", "kind": "Task", "description": "A" },
                        { "id": "b", "kind": "Task", "description": "B" },
                        { "id": "c", "kind": "Task", "description": "C" }
                    ],
                    "edges": [
                        { "from": "a", "to": "b" },
                        { "from": "b", "to": "c" },
                        { "from": "c", "to": "a" }
                    ]
                }
            }
        });

        let result = validator.validate_json(&cyclic_plan);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(e, ValidationError::CyclicGraph { .. })));
    }

    #[test]
    fn invalid_edge_reference_fails() {
        let validator = Validator::new().unwrap();
        let plan = json!({
            "apiVersion": "planspec.io/v1alpha1",
            "kind": "Plan",
            "metadata": {
                "name": "bad-edges",
                "namespace": "default"
            },
            "spec": {
                "description": "Plan with bad edge",
                "graph": {
                    "nodes": [
                        { "id": "task-1", "kind": "Task", "description": "Task 1" }
                    ],
                    "edges": [
                        { "from": "task-1", "to": "nonexistent" }
                    ]
                }
            }
        });

        let result = validator.validate_json(&plan);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(e, ValidationError::InvalidEdgeReference { .. })));
    }
}

mod graph {
    use super::*;

    fn make_graph(nodes: Vec<(&str, &str)>, edges: Vec<(&str, &str)>) -> Graph {
        Graph {
            nodes: nodes
                .into_iter()
                .map(|(id, desc)| Node::task(id, desc))
                .collect(),
            edges: edges
                .into_iter()
                .map(|(from, to)| Edge::new(from, to))
                .collect(),
        }
    }

    #[test]
    fn empty_graph_is_acyclic() {
        let graph = Graph::default();
        assert!(graph.is_acyclic());
    }

    #[test]
    fn single_node_is_acyclic() {
        let graph = make_graph(vec![("a", "Task A")], vec![]);
        assert!(graph.is_acyclic());
    }

    #[test]
    fn linear_chain_is_acyclic() {
        let graph = make_graph(
            vec![("a", "A"), ("b", "B"), ("c", "C")],
            vec![("a", "b"), ("b", "c")],
        );
        assert!(graph.is_acyclic());
    }

    #[test]
    fn diamond_is_acyclic() {
        let graph = make_graph(
            vec![("a", "A"), ("b", "B"), ("c", "C"), ("d", "D")],
            vec![("a", "b"), ("a", "c"), ("b", "d"), ("c", "d")],
        );
        assert!(graph.is_acyclic());
    }

    #[test]
    fn simple_cycle_detected() {
        let graph = make_graph(
            vec![("a", "A"), ("b", "B")],
            vec![("a", "b"), ("b", "a")],
        );
        assert!(!graph.is_acyclic());
        assert!(graph.detect_cycle().is_some());
    }

    #[test]
    fn self_loop_detected() {
        let graph = make_graph(
            vec![("a", "A")],
            vec![("a", "a")],
        );
        assert!(!graph.is_acyclic());
    }

    #[test]
    fn complex_cycle_detected() {
        let graph = make_graph(
            vec![("a", "A"), ("b", "B"), ("c", "C"), ("d", "D")],
            vec![("a", "b"), ("b", "c"), ("c", "d"), ("d", "b")],
        );
        assert!(!graph.is_acyclic());
    }

    #[test]
    fn roots_are_correct() {
        let graph = make_graph(
            vec![("a", "A"), ("b", "B"), ("c", "C")],
            vec![("a", "b"), ("a", "c")],
        );
        let roots = graph.roots();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, "a");
    }

    #[test]
    fn leaves_are_correct() {
        let graph = make_graph(
            vec![("a", "A"), ("b", "B"), ("c", "C")],
            vec![("a", "b"), ("a", "c")],
        );
        let leaves = graph.leaves();
        assert_eq!(leaves.len(), 2);
        let leaf_ids: Vec<_> = leaves.iter().map(|n| n.id.as_str()).collect();
        assert!(leaf_ids.contains(&"b"));
        assert!(leaf_ids.contains(&"c"));
    }

    #[test]
    fn topological_order_is_valid() {
        let graph = make_graph(
            vec![("a", "A"), ("b", "B"), ("c", "C")],
            vec![("a", "b"), ("b", "c")],
        );
        let order = graph.topological_order().unwrap();
        let ids: Vec<_> = order.iter().map(|n| n.id.as_str()).collect();

        // a must come before b, b must come before c
        let pos_a = ids.iter().position(|&id| id == "a").unwrap();
        let pos_b = ids.iter().position(|&id| id == "b").unwrap();
        let pos_c = ids.iter().position(|&id| id == "c").unwrap();

        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn topological_order_fails_on_cycle() {
        let graph = make_graph(
            vec![("a", "A"), ("b", "B")],
            vec![("a", "b"), ("b", "a")],
        );
        assert!(graph.topological_order().is_err());
    }
}

mod builders {
    use super::*;

    #[test]
    fn object_meta_builder() {
        let meta = ObjectMeta::new("test", "default")
            .with_label("app", "myapp")
            .with_label("env", "prod")
            .with_annotation("note", "important");

        assert_eq!(meta.name, "test");
        assert_eq!(meta.namespace, "default");
        assert_eq!(meta.labels.get("app"), Some(&"myapp".to_string()));
        assert_eq!(meta.labels.get("env"), Some(&"prod".to_string()));
        assert_eq!(meta.annotations.get("note"), Some(&"important".to_string()));
    }

    #[test]
    fn goal_builder() {
        let goal = Goal::new("my-goal", "default")
            .with_description("Do something")
            .with_criterion("It works")
            .with_criterion("It's fast")
            .with_timeout("24h")
            .with_priority(100);

        assert_eq!(goal.metadata.name, "my-goal");
        assert_eq!(goal.spec.description, "Do something");
        assert_eq!(goal.spec.acceptance_criteria.len(), 2);
        assert_eq!(goal.spec.timeout, Some("24h".to_string()));
        assert_eq!(goal.spec.priority, Some(100));
    }

    #[test]
    fn plan_builder() {
        let plan = Plan::new("my-plan", "default")
            .with_description("Implementation plan")
            .with_goal_ref("my-goal")
            .with_series("implementation")
            .with_version("1");

        assert_eq!(plan.metadata.name, "my-plan");
        assert_eq!(plan.spec.description, "Implementation plan");
        assert_eq!(plan.spec.goal_ref.unwrap().name, "my-goal");
        assert_eq!(plan.spec.series, Some("implementation".to_string()));
        assert_eq!(plan.spec.version, Some("1".to_string()));
    }

    #[test]
    fn node_builder() {
        let node = Node::task("my-task", "Do the thing")
            .with_capability("code-gen")
            .with_when("always");

        assert_eq!(node.id, "my-task");
        assert_eq!(node.kind, NodeKind::Task);
        assert_eq!(node.description, "Do the thing");
        assert_eq!(node.capability_ref.unwrap().name, "code-gen");
        assert_eq!(node.when, Some("always".to_string()));
    }

    #[test]
    fn edge_builder() {
        let hard = Edge::new("a", "b");
        assert_eq!(hard.from, "a");
        assert_eq!(hard.to, "b");
        assert!(hard.edge_type.is_none());

        let soft = Edge::soft("c", "d");
        assert_eq!(soft.edge_type, Some(EdgeType::Soft));
    }

    #[test]
    fn binding_rule_builders() {
        let cap_rule = BindingRule::for_capability("code-gen", "agent://claude");
        assert_eq!(cap_rule.selector.capability_ref.unwrap().name, "code-gen");
        assert_eq!(cap_rule.target.provider, "agent://claude");

        let node_rule = BindingRule::for_node("my-plan", "task-1", "agent://gpt");
        assert_eq!(node_rule.selector.plan_ref.unwrap().name, "my-plan");
        assert_eq!(node_rule.selector.node_id, Some("task-1".to_string()));
        assert_eq!(node_rule.target.provider, "agent://gpt");
    }
}

mod api_types {
    use super::*;

    #[test]
    fn watch_event_builders() {
        let obj = json!({"kind": "Goal", "metadata": {"name": "test"}});

        let added = WatchEvent::added(obj.clone(), "100");
        assert_eq!(added.event_type, WatchEventType::Added);
        assert_eq!(added.rev, "100");

        let modified = WatchEvent::modified(obj.clone(), "101");
        assert_eq!(modified.event_type, WatchEventType::Modified);

        let deleted = WatchEvent::deleted(obj, "102");
        assert_eq!(deleted.event_type, WatchEventType::Deleted);
    }

    #[test]
    fn status_builders() {
        let not_found = Status::not_found("Goal", "my-goal");
        assert_eq!(not_found.code, 404);
        assert_eq!(not_found.reason, Some(StatusReason::NotFound));
        assert_eq!(not_found.status, StatusResult::Failure);

        let invalid = Status::invalid("Bad data");
        assert_eq!(invalid.code, 400);
        assert_eq!(invalid.reason, Some(StatusReason::Invalid));

        let conflict = Status::conflict("Version mismatch");
        assert_eq!(conflict.code, 409);
        assert_eq!(conflict.reason, Some(StatusReason::Conflict));

        let success = Status::success("Created", 201);
        assert_eq!(success.code, 201);
        assert_eq!(success.status, StatusResult::Success);
    }

    #[test]
    fn resource_list_builder() {
        let goals = vec![
            Goal::new("goal-1", "default"),
            Goal::new("goal-2", "default"),
        ];

        let list = ResourceList::new("GoalList", goals);
        assert_eq!(list.kind, "GoalList");
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.api_version, API_VERSION);
    }

    #[test]
    fn apply_result() {
        let mut result = ApplyResult::new();
        assert!(result.is_success());

        result.errors.push(ApplyError {
            resource: "Goal/bad".to_string(),
            message: "Invalid".to_string(),
        });
        assert!(!result.is_success());
    }
}
