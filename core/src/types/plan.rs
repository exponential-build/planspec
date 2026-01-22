//! Plan resource type - DAGs of work with embedded nodes and edges.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::context::ContextItem;
use super::meta::{Condition, ObjectMeta, ObjectReference};
use crate::API_VERSION;

/// Machine-verifiable acceptance criteria for Task nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AcceptanceCriteria {
    /// Check that a file or directory exists.
    ArtifactExists {
        /// Human-readable name for this criterion.
        name: String,
        /// Detailed description of what this criterion validates.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Whether this criterion must pass for task completion.
        #[serde(default = "default_required")]
        required: bool,
        /// File or directory path to check.
        path: String,
        /// Optional regex pattern that must match file content.
        #[serde(skip_serializing_if = "Option::is_none")]
        content_match: Option<String>,
    },
    /// Run tests and check they pass.
    TestPasses {
        /// Human-readable name for this criterion.
        name: String,
        /// Detailed description of what this criterion validates.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Whether this criterion must pass for task completion.
        #[serde(default = "default_required")]
        required: bool,
        /// Command to run.
        command: String,
        /// Arguments to pass to the command.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        /// Expected exit code (default: 0).
        #[serde(default)]
        expected_exit_code: i32,
        /// Maximum execution time (Go duration format).
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<String>,
    },
    /// Check that an HTTP endpoint responds correctly.
    EndpointResponds {
        /// Human-readable name for this criterion.
        name: String,
        /// Detailed description of what this criterion validates.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Whether this criterion must pass for task completion.
        #[serde(default = "default_required")]
        required: bool,
        /// URL to check.
        url: String,
        /// HTTP method (default: GET).
        #[serde(default = "default_method")]
        method: String,
        /// Expected HTTP status code (default: 200).
        #[serde(default = "default_status_code")]
        expected_status: i32,
        /// Optional regex pattern that must match response body.
        #[serde(skip_serializing_if = "Option::is_none")]
        body_match: Option<String>,
        /// Maximum wait time (Go duration format).
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<String>,
    },
    /// Run a command and check it succeeds.
    CommandSucceeds {
        /// Human-readable name for this criterion.
        name: String,
        /// Detailed description of what this criterion validates.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Whether this criterion must pass for task completion.
        #[serde(default = "default_required")]
        required: bool,
        /// Command to run.
        command: String,
        /// Arguments to pass to the command.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        /// Expected exit code (default: 0).
        #[serde(default)]
        expected_exit_code: i32,
        /// Optional regex pattern that must match stdout.
        #[serde(skip_serializing_if = "Option::is_none")]
        output_match: Option<String>,
        /// Maximum execution time (Go duration format).
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<String>,
    },
    /// Call an external webhook for validation.
    Custom {
        /// Human-readable name for this criterion.
        name: String,
        /// Detailed description of what this criterion validates.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Whether this criterion must pass for task completion.
        #[serde(default = "default_required")]
        required: bool,
        /// URL to call for validation.
        webhook_url: String,
        /// JSON payload template to send.
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<Value>,
        /// Expected response criteria.
        #[serde(skip_serializing_if = "Option::is_none")]
        expected_response: Option<ExpectedResponse>,
        /// Maximum wait time (Go duration format).
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<String>,
    },
}

fn default_required() -> bool {
    true
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_status_code() -> i32 {
    200
}

/// Expected response for custom webhook validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedResponse {
    /// Expected HTTP status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<i32>,
    /// Regex pattern for response body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_match: Option<String>,
}

/// Plan represents a directed acyclic graph of work to be executed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    /// API version, always "planspec.io/v1alpha1".
    pub api_version: String,

    /// Kind, always "Plan".
    pub kind: String,

    /// Standard object metadata.
    pub metadata: ObjectMeta,

    /// Specification of the plan.
    pub spec: PlanSpec,

    /// Current status of the plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PlanStatus>,
}

impl Default for Plan {
    fn default() -> Self {
        Self {
            api_version: API_VERSION.to_string(),
            kind: "Plan".to_string(),
            metadata: ObjectMeta::default(),
            spec: PlanSpec::default(),
            status: None,
        }
    }
}

impl Plan {
    /// Create a new Plan with the given name and namespace.
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            metadata: ObjectMeta::new(name, namespace),
            ..Default::default()
        }
    }

    /// Set the description of this plan.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.spec.description = description.into();
        self
    }

    /// Set the goal reference for this plan.
    pub fn with_goal_ref(mut self, goal_name: impl Into<String>) -> Self {
        self.spec.goal_ref = Some(ObjectReference::new(goal_name));
        self
    }

    /// Set the series for this plan.
    pub fn with_series(mut self, series: impl Into<String>) -> Self {
        self.spec.series = Some(series.into());
        self
    }

    /// Set the version for this plan.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.spec.version = Some(version.into());
        self
    }

    /// Get the number of nodes in this plan.
    pub fn node_count(&self) -> usize {
        self.spec.graph.nodes.len()
    }

    /// Get the number of edges in this plan.
    pub fn edge_count(&self) -> usize {
        self.spec.graph.edges.len()
    }
}

/// PlanSpec defines the desired state of a Plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlanSpec {
    /// Human-readable description of the plan.
    pub description: String,

    /// Reference to the goal this plan achieves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal_ref: Option<ObjectReference>,

    /// Stable identifier for this plan family.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,

    /// Version within the series.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Plans that this plan supersedes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<ObjectReference>,

    /// Additional context for executing this plan.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<ContextItem>,

    /// The directed acyclic graph of work.
    pub graph: Graph,
}

/// Graph represents the DAG structure of a plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Graph {
    /// Nodes in the graph.
    pub nodes: Vec<Node>,

    /// Edges connecting nodes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<Edge>,
}

impl Graph {
    /// Check if the graph is acyclic.
    pub fn is_acyclic(&self) -> bool {
        self.detect_cycle().is_none()
    }

    /// Detect a cycle in the graph, returning the first node ID in the cycle if found.
    pub fn detect_cycle(&self) -> Option<String> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        // Build adjacency list
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &self.nodes {
            adj.entry(&node.id).or_default();
        }
        for edge in &self.edges {
            adj.entry(&edge.from).or_default().push(&edge.to);
        }

        for node in &self.nodes {
            if let Some(cycle_node) = self.dfs_cycle(&node.id, &adj, &mut visited, &mut rec_stack) {
                return Some(cycle_node);
            }
        }

        None
    }

    fn dfs_cycle<'a>(
        &self,
        node: &'a str,
        adj: &HashMap<&str, Vec<&'a str>>,
        visited: &mut HashSet<&'a str>,
        rec_stack: &mut HashSet<&'a str>,
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
                if let Some(cycle) = self.dfs_cycle(neighbor, adj, visited, rec_stack) {
                    return Some(cycle);
                }
            }
        }

        rec_stack.remove(node);
        None
    }

    /// Get nodes with no incoming edges (roots).
    pub fn roots(&self) -> Vec<&Node> {
        let has_incoming: HashSet<&str> = self.edges.iter().map(|e| e.to.as_str()).collect();
        self.nodes
            .iter()
            .filter(|n| !has_incoming.contains(n.id.as_str()))
            .collect()
    }

    /// Get nodes with no outgoing edges (leaves).
    pub fn leaves(&self) -> Vec<&Node> {
        let has_outgoing: HashSet<&str> = self.edges.iter().map(|e| e.from.as_str()).collect();
        self.nodes
            .iter()
            .filter(|n| !has_outgoing.contains(n.id.as_str()))
            .collect()
    }

    /// Get a topological ordering of nodes, or error if cyclic.
    pub fn topological_order(&self) -> Result<Vec<&Node>, GraphError> {
        if let Some(cycle_node) = self.detect_cycle() {
            return Err(GraphError::CyclicGraph {
                node_id: cycle_node,
            });
        }

        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();

        // Build adjacency list
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &self.nodes {
            adj.entry(&node.id).or_default();
        }
        for edge in &self.edges {
            adj.entry(&edge.from).or_default().push(&edge.to);
        }

        // Build node lookup
        let node_map: HashMap<&str, &Node> =
            self.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

        for node in &self.nodes {
            if !visited.contains(node.id.as_str()) {
                self.topo_visit(
                    &node.id,
                    &adj,
                    &node_map,
                    &mut visited,
                    &mut temp_visited,
                    &mut result,
                )?;
            }
        }

        result.reverse();
        Ok(result)
    }

    fn topo_visit<'a>(
        &self,
        node_id: &'a str,
        adj: &HashMap<&str, Vec<&'a str>>,
        node_map: &HashMap<&str, &'a Node>,
        visited: &mut HashSet<&'a str>,
        temp_visited: &mut HashSet<&'a str>,
        result: &mut Vec<&'a Node>,
    ) -> Result<(), GraphError> {
        if temp_visited.contains(node_id) {
            return Err(GraphError::CyclicGraph {
                node_id: node_id.to_string(),
            });
        }
        if visited.contains(node_id) {
            return Ok(());
        }

        temp_visited.insert(node_id);

        if let Some(neighbors) = adj.get(node_id) {
            for neighbor in neighbors {
                self.topo_visit(neighbor, adj, node_map, visited, temp_visited, result)?;
            }
        }

        temp_visited.remove(node_id);
        visited.insert(node_id);
        if let Some(node) = node_map.get(node_id) {
            result.push(node);
        }

        Ok(())
    }
}

/// GraphError represents errors in graph operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GraphError {
    /// The graph contains a cycle.
    #[error("graph contains a cycle involving node '{node_id}'")]
    CyclicGraph { node_id: String },

    /// A referenced node does not exist.
    #[error("node '{node_id}' not found")]
    NodeNotFound { node_id: String },
}

/// Node represents a unit of work in the plan graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// Unique identifier within the plan.
    pub id: String,

    /// Type of node.
    pub kind: NodeKind,

    /// Human-readable name for this node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Human-readable description of the work.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// References to capabilities required for this node (Task nodes).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capability_refs: Vec<ObjectReference>,

    /// Input parameters for this node (Task nodes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Value>,

    /// Expected output artifacts (Task nodes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<String>>,

    /// Maximum execution time for this node (Task/External nodes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,

    /// Number of retry attempts on failure (Task nodes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<i32>,

    /// Condition for executing this node (Task nodes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,

    /// Additional context for executing this node.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<ContextItem>,

    /// Reference to a Gate resource (required for Gate nodes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate_ref: Option<ObjectReference>,

    /// Machine-verifiable acceptance criteria (Task nodes).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acceptance_criteria: Vec<AcceptanceCriteria>,

    /// Estimated effort level (Task nodes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_effort: Option<EstimatedEffort>,

    /// List of node IDs this node depends on (convenience, normalized to edges).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,

    /// Child node IDs (required for Group nodes).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<String>,

    /// Execution mode for children (Group nodes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<GroupMode>,

    /// Reference to external system (required for External nodes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<ExternalRef>,

    /// Poll interval for external status checks (External nodes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_interval: Option<String>,
}

/// Execution mode for Group node children.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupMode {
    /// Children execute in parallel.
    Parallel,
    /// Children execute in sequence.
    Sequence,
}

/// Reference to an external system or process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalRef {
    /// Type of external reference.
    #[serde(rename = "type")]
    pub ref_type: ExternalRefType,

    /// URI of the external system (for type: uri).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,

    /// Reference to a PlanSpec resource (for type: resource).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_ref: Option<ObjectReference>,

    /// Webhook URL to poll or call (for type: webhook).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
}

/// Type of external reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExternalRefType {
    /// External URI.
    Uri,
    /// Reference to another PlanSpec resource.
    Resource,
    /// Webhook to poll.
    Webhook,
}

/// Estimated effort level for a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EstimatedEffort {
    Small,
    Medium,
    Large,
    Xlarge,
}

impl Node {
    /// Create a new Task node.
    pub fn task(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: NodeKind::Task,
            name: None,
            description: Some(description.into()),
            capability_refs: Vec::new(),
            inputs: None,
            outputs: None,
            timeout: None,
            retries: None,
            when: None,
            context: Vec::new(),
            gate_ref: None,
            acceptance_criteria: Vec::new(),
            estimated_effort: None,
            depends_on: Vec::new(),
            children: Vec::new(),
            mode: None,
            external_ref: None,
            poll_interval: None,
        }
    }

    /// Create a new Gate node.
    pub fn gate(id: impl Into<String>, gate_ref: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: NodeKind::Gate,
            name: None,
            description: None,
            capability_refs: Vec::new(),
            inputs: None,
            outputs: None,
            timeout: None,
            retries: None,
            when: None,
            context: Vec::new(),
            gate_ref: Some(ObjectReference::new(gate_ref)),
            acceptance_criteria: Vec::new(),
            estimated_effort: None,
            depends_on: Vec::new(),
            children: Vec::new(),
            mode: None,
            external_ref: None,
            poll_interval: None,
        }
    }

    /// Create a new Group node.
    pub fn group(id: impl Into<String>, children: Vec<String>) -> Self {
        Self {
            id: id.into(),
            kind: NodeKind::Group,
            name: None,
            description: None,
            capability_refs: Vec::new(),
            inputs: None,
            outputs: None,
            timeout: None,
            retries: None,
            when: None,
            context: Vec::new(),
            gate_ref: None,
            acceptance_criteria: Vec::new(),
            estimated_effort: None,
            depends_on: Vec::new(),
            children,
            mode: None,
            external_ref: None,
            poll_interval: None,
        }
    }

    /// Create a new External node.
    pub fn external(id: impl Into<String>, external_ref: ExternalRef) -> Self {
        Self {
            id: id.into(),
            kind: NodeKind::External,
            name: None,
            description: None,
            capability_refs: Vec::new(),
            inputs: None,
            outputs: None,
            timeout: None,
            retries: None,
            when: None,
            context: Vec::new(),
            gate_ref: None,
            acceptance_criteria: Vec::new(),
            estimated_effort: None,
            depends_on: Vec::new(),
            children: Vec::new(),
            mode: None,
            external_ref: Some(external_ref),
            poll_interval: None,
        }
    }

    /// Add a capability reference to this node.
    pub fn with_capability(mut self, capability_name: impl Into<String>) -> Self {
        self.capability_refs.push(ObjectReference::new(capability_name));
        self
    }

    /// Set the when condition for this node.
    pub fn with_when(mut self, condition: impl Into<String>) -> Self {
        self.when = Some(condition.into());
        self
    }

    /// Set the description for this node.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the group mode for this node.
    pub fn with_mode(mut self, mode: GroupMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Add acceptance criteria to this node.
    pub fn with_acceptance_criteria(mut self, criteria: AcceptanceCriteria) -> Self {
        self.acceptance_criteria.push(criteria);
        self
    }
}

/// NodeKind represents the type of a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeKind {
    /// A unit of work to be executed.
    Task,
    /// A synchronization point or approval gate.
    Gate,
    /// A container for nested nodes.
    Group,
    /// An externally-managed node.
    External,
}

/// Edge represents a dependency between nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    /// Source node ID.
    pub from: String,

    /// Target node ID.
    pub to: String,

    /// Type of edge (defaults to hard dependency).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub edge_type: Option<EdgeType>,
}

impl Edge {
    /// Create a new edge from one node to another.
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            edge_type: None,
        }
    }

    /// Create a soft edge (non-blocking dependency).
    pub fn soft(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            edge_type: Some(EdgeType::Soft),
        }
    }
}

/// EdgeType represents the type of dependency between nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeType {
    /// Hard dependency - target cannot start until source completes.
    Hard,
    /// Soft dependency - target may start before source completes.
    Soft,
}

/// PlanStatus represents the current state of a Plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlanStatus {
    /// Current phase of the plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<PlanPhase>,

    /// Number of nodes in the plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_count: Option<i64>,

    /// The generation most recently observed by the controller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_generation: Option<i64>,

    /// Current conditions of the plan.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
}

/// PlanPhase represents the lifecycle phase of a Plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlanPhase {
    /// Plan is valid and ready for execution.
    Ready,
    /// Plan failed validation.
    Invalid,
}
