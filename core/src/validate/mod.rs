//! Validation for PlanSpec resources.

mod schemas;

use jsonschema::JSONSchema;
use serde_json::Value;

use crate::types::plan::{Graph, GraphError, Plan};
use crate::Resource;

/// Validator for PlanSpec resources.
pub struct Validator {
    goal_schema: JSONSchema,
    plan_schema: JSONSchema,
    capability_schema: JSONSchema,
    binding_schema: JSONSchema,
    execution_schema: JSONSchema,
}

impl Validator {
    /// Create a new validator with embedded schemas.
    pub fn new() -> Result<Self, ValidationError> {
        Ok(Self {
            goal_schema: compile_schema(schemas::GOAL_SCHEMA)?,
            plan_schema: compile_schema(schemas::PLAN_SCHEMA)?,
            capability_schema: compile_schema(schemas::CAPABILITY_SCHEMA)?,
            binding_schema: compile_schema(schemas::BINDING_SCHEMA)?,
            execution_schema: compile_schema(schemas::EXECUTION_SCHEMA)?,
        })
    }

    /// Validate a resource.
    pub fn validate(&self, resource: &Resource) -> Result<(), Vec<ValidationError>> {
        let value = serde_json::to_value(resource)
            .map_err(|e| vec![ValidationError::SerializationError(e.to_string())])?;

        self.validate_json(&value)?;

        // Additional validation for Plans
        if let Resource::Plan(plan) = resource {
            self.validate_plan_graph(plan)?;
        }

        Ok(())
    }

    /// Validate a JSON value as a resource.
    pub fn validate_json(&self, value: &Value) -> Result<(), Vec<ValidationError>> {
        let kind = value
            .get("kind")
            .and_then(|k| k.as_str())
            .ok_or_else(|| vec![ValidationError::MissingKind])?;

        let schema = match kind {
            "Goal" => &self.goal_schema,
            "Plan" => &self.plan_schema,
            "Capability" => &self.capability_schema,
            "Binding" => &self.binding_schema,
            "Execution" => &self.execution_schema,
            _ => return Err(vec![ValidationError::UnknownKind(kind.to_string())]),
        };

        let result = schema.validate(value);

        if let Err(errors) = result {
            let error_list: Vec<ValidationError> = errors
                .map(|e| {
                    let path = e.instance_path.to_string();
                    let path_str = if path.is_empty() {
                        "(root)".to_string()
                    } else {
                        path
                    };
                    ValidationError::SchemaValidation {
                        path: path_str,
                        message: e.to_string(),
                    }
                })
                .collect();

            if error_list.is_empty() {
                Ok(())
            } else {
                Err(error_list)
            }
        } else {
            // Validate naming conventions
            self.validate_naming_conventions(value)?;

            // For Plans, also check for cycles and node IDs
            if kind == "Plan" {
                self.validate_plan_graph_json(value)?;
                self.validate_plan_node_ids(value)?;
            }
            Ok(())
        }
    }

    /// Validate naming conventions for metadata.name and metadata.namespace.
    fn validate_naming_conventions(&self, value: &Value) -> Result<(), Vec<ValidationError>> {
        let metadata = value.get("metadata");

        if let Some(meta) = metadata {
            // Validate name
            if let Some(name) = meta.get("name").and_then(|n| n.as_str()) {
                if !is_valid_dns_label(name) {
                    return Err(vec![ValidationError::InvalidName {
                        field: "metadata.name".to_string(),
                        value: name.to_string(),
                    }]);
                }
            }

            // Validate namespace
            if let Some(namespace) = meta.get("namespace").and_then(|n| n.as_str()) {
                if !is_valid_dns_label(namespace) {
                    return Err(vec![ValidationError::InvalidName {
                        field: "metadata.namespace".to_string(),
                        value: namespace.to_string(),
                    }]);
                }
            }
        }

        Ok(())
    }

    /// Validate Plan node IDs follow naming conventions.
    fn validate_plan_node_ids(&self, value: &Value) -> Result<(), Vec<ValidationError>> {
        let nodes = value
            .get("spec")
            .and_then(|s| s.get("graph"))
            .and_then(|g| g.get("nodes"))
            .and_then(|n| n.as_array());

        if let Some(nodes) = nodes {
            for node in nodes {
                if let Some(id) = node.get("id").and_then(|i| i.as_str()) {
                    if !is_valid_node_id(id) {
                        return Err(vec![ValidationError::InvalidName {
                            field: "spec.graph.nodes[].id".to_string(),
                            value: id.to_string(),
                        }]);
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate a Plan's graph for cycles and other constraints.
    fn validate_plan_graph(&self, plan: &Plan) -> Result<(), Vec<ValidationError>> {
        if let Some(cycle_node) = plan.spec.graph.detect_cycle() {
            return Err(vec![ValidationError::CyclicGraph {
                node_id: cycle_node,
            }]);
        }

        // Validate that edge references exist
        self.validate_edge_references(&plan.spec.graph)?;

        Ok(())
    }

    /// Validate a Plan's graph from JSON.
    fn validate_plan_graph_json(&self, value: &Value) -> Result<(), Vec<ValidationError>> {
        let graph = value.get("spec").and_then(|s| s.get("graph"));

        if let Some(graph_value) = graph {
            let nodes = graph_value
                .get("nodes")
                .and_then(|n| n.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|n| n.get("id").and_then(|id| id.as_str()))
                        .collect::<std::collections::HashSet<_>>()
                })
                .unwrap_or_default();

            let empty_edges = vec![];
            let edges = graph_value
                .get("edges")
                .and_then(|e| e.as_array())
                .unwrap_or(&empty_edges);

            // Check for invalid edge references
            for edge in edges {
                let from = edge.get("from").and_then(|f| f.as_str()).unwrap_or("");
                let to = edge.get("to").and_then(|t| t.as_str()).unwrap_or("");

                if !nodes.contains(from) {
                    return Err(vec![ValidationError::InvalidEdgeReference {
                        edge_field: "from".to_string(),
                        node_id: from.to_string(),
                    }]);
                }
                if !nodes.contains(to) {
                    return Err(vec![ValidationError::InvalidEdgeReference {
                        edge_field: "to".to_string(),
                        node_id: to.to_string(),
                    }]);
                }
            }

            // Check for cycles using DFS
            let mut adj: std::collections::HashMap<&str, Vec<&str>> =
                std::collections::HashMap::new();
            for node_id in &nodes {
                adj.entry(node_id).or_default();
            }
            for edge in edges {
                let from = edge.get("from").and_then(|f| f.as_str()).unwrap_or("");
                let to = edge.get("to").and_then(|t| t.as_str()).unwrap_or("");
                adj.entry(from).or_default().push(to);
            }

            let mut visited = std::collections::HashSet::new();
            let mut rec_stack = std::collections::HashSet::new();

            for node_id in &nodes {
                if let Some(cycle) = detect_cycle_dfs(node_id, &adj, &mut visited, &mut rec_stack) {
                    return Err(vec![ValidationError::CyclicGraph { node_id: cycle }]);
                }
            }
        }

        Ok(())
    }

    /// Validate that all edge references point to existing nodes.
    fn validate_edge_references(&self, graph: &Graph) -> Result<(), Vec<ValidationError>> {
        let node_ids: std::collections::HashSet<&str> =
            graph.nodes.iter().map(|n| n.id.as_str()).collect();

        for edge in &graph.edges {
            if !node_ids.contains(edge.from.as_str()) {
                return Err(vec![ValidationError::InvalidEdgeReference {
                    edge_field: "from".to_string(),
                    node_id: edge.from.clone(),
                }]);
            }
            if !node_ids.contains(edge.to.as_str()) {
                return Err(vec![ValidationError::InvalidEdgeReference {
                    edge_field: "to".to_string(),
                    node_id: edge.to.clone(),
                }]);
            }
        }

        Ok(())
    }
}

fn detect_cycle_dfs<'a>(
    node: &'a str,
    adj: &std::collections::HashMap<&str, Vec<&'a str>>,
    visited: &mut std::collections::HashSet<&'a str>,
    rec_stack: &mut std::collections::HashSet<&'a str>,
) -> Option<String> {
    if rec_stack.contains(node) {
        return Some(node.to_string());
    }
    if visited.contains(node) {
        return None;
    }

    visited.insert(node);
    rec_stack.insert(node);

    if let Some(neighbors) = adj.get(node) {
        for neighbor in neighbors {
            if let Some(cycle) = detect_cycle_dfs(neighbor, adj, visited, rec_stack) {
                return Some(cycle);
            }
        }
    }

    rec_stack.remove(node);
    None
}

fn compile_schema(schema_json: &str) -> Result<JSONSchema, ValidationError> {
    let schema: Value = serde_json::from_str(schema_json).map_err(|e| {
        ValidationError::SchemaCompilationError(format!("Failed to parse schema: {}", e))
    })?;

    JSONSchema::compile(&schema).map_err(|e| {
        ValidationError::SchemaCompilationError(format!("Failed to compile schema: {}", e))
    })
}

/// Check if a string is a valid DNS label (Kubernetes naming convention).
/// Must be lowercase, start with a letter, contain only letters, numbers, and hyphens.
/// Maximum 63 characters.
fn is_valid_dns_label(s: &str) -> bool {
    if s.is_empty() || s.len() > 63 {
        return false;
    }

    let mut chars = s.chars().peekable();

    // Must start with a lowercase letter
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }

    // Rest must be lowercase letters, digits, or hyphens
    for c in chars {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
            return false;
        }
    }

    // Must not end with a hyphen
    if s.ends_with('-') {
        return false;
    }

    true
}

/// Check if a string is a valid node ID.
/// Similar to DNS labels but slightly more permissive - allows underscores.
fn is_valid_node_id(s: &str) -> bool {
    if s.is_empty() || s.len() > 63 {
        return false;
    }

    let mut chars = s.chars().peekable();

    // Must start with a lowercase letter
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }

    // Rest must be lowercase letters, digits, hyphens, or underscores
    for c in chars {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' && c != '_' {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod naming_tests {
    use super::*;

    #[test]
    fn test_valid_dns_labels() {
        assert!(is_valid_dns_label("my-goal"));
        assert!(is_valid_dns_label("planspec"));
        assert!(is_valid_dns_label("test-123"));
        assert!(is_valid_dns_label("a"));
        assert!(is_valid_dns_label("abc123"));
    }

    #[test]
    fn test_invalid_dns_labels() {
        assert!(!is_valid_dns_label("")); // empty
        assert!(!is_valid_dns_label("My-Goal")); // uppercase
        assert!(!is_valid_dns_label("123-test")); // starts with number
        assert!(!is_valid_dns_label("-test")); // starts with hyphen
        assert!(!is_valid_dns_label("test-")); // ends with hyphen
        assert!(!is_valid_dns_label("test_name")); // underscore not allowed
        assert!(!is_valid_dns_label("test.name")); // dot not allowed
    }

    #[test]
    fn test_valid_node_ids() {
        assert!(is_valid_node_id("step-1"));
        assert!(is_valid_node_id("my_task"));
        assert!(is_valid_node_id("schema-and-conventions"));
        assert!(is_valid_node_id("v0-ready"));
    }

    #[test]
    fn test_invalid_node_ids() {
        assert!(!is_valid_node_id("")); // empty
        assert!(!is_valid_node_id("Step-1")); // uppercase
        assert!(!is_valid_node_id("1-step")); // starts with number
        assert!(!is_valid_node_id("step.one")); // dot not allowed
    }
}

/// Error during validation.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    /// Missing kind field.
    #[error("missing 'kind' field")]
    MissingKind,

    /// Unknown resource kind.
    #[error("unknown resource kind: {0}")]
    UnknownKind(String),

    /// Schema validation failed.
    #[error("validation failed at {path}: {message}")]
    SchemaValidation { path: String, message: String },

    /// Schema compilation error.
    #[error("schema compilation error: {0}")]
    SchemaCompilationError(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// Graph contains a cycle.
    #[error("graph contains a cycle involving node '{node_id}'")]
    CyclicGraph { node_id: String },

    /// Invalid edge reference.
    #[error("edge '{edge_field}' references non-existent node '{node_id}'")]
    InvalidEdgeReference { edge_field: String, node_id: String },

    /// Invalid resource name (must be lowercase, alphanumeric, hyphens only).
    #[error("invalid {field}: '{value}' - must be lowercase, start with letter, contain only letters, numbers, and hyphens")]
    InvalidName { field: String, value: String },
}

impl From<GraphError> for ValidationError {
    fn from(err: GraphError) -> Self {
        match err {
            GraphError::CyclicGraph { node_id } => ValidationError::CyclicGraph { node_id },
            GraphError::NodeNotFound { node_id } => ValidationError::InvalidEdgeReference {
                edge_field: "unknown".to_string(),
                node_id,
            },
        }
    }
}
