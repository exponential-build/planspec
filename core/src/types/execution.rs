//! Execution resource type - intent to run a Plan with specific bindings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use super::context::ContextItem;
use super::meta::{Condition, ObjectMeta, ObjectReference};
use crate::API_VERSION;

/// Execution represents the intent to run a Plan with specific bindings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Execution {
    /// API version, always "planspec.io/v1alpha1".
    pub api_version: String,

    /// Kind, always "Execution".
    pub kind: String,

    /// Standard object metadata.
    pub metadata: ObjectMeta,

    /// Specification of the execution (immutable after creation).
    pub spec: ExecutionSpec,

    /// Current status of the execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ExecutionStatus>,
}

impl Default for Execution {
    fn default() -> Self {
        Self {
            api_version: API_VERSION.to_string(),
            kind: "Execution".to_string(),
            metadata: ObjectMeta::default(),
            spec: ExecutionSpec::default(),
            status: None,
        }
    }
}

impl Execution {
    /// Create a new Execution with the given name and namespace.
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            metadata: ObjectMeta::new(name, namespace),
            ..Default::default()
        }
    }

    /// Set the plan reference for this execution.
    pub fn with_plan_ref(mut self, plan_name: impl Into<String>) -> Self {
        self.spec.plan_ref = ObjectReference::new(plan_name);
        self
    }

    /// Set the goal reference for this execution.
    pub fn with_goal_ref(mut self, goal_name: impl Into<String>) -> Self {
        self.spec.goal_ref = Some(ObjectReference::new(goal_name));
        self
    }

    /// Set the binding reference for this execution.
    pub fn with_binding_ref(mut self, binding_name: impl Into<String>) -> Self {
        self.spec.binding_ref = Some(ObjectReference::new(binding_name));
        self
    }
}

/// ExecutionSpec defines the desired state of an Execution.
/// This is immutable after creation - the intent is fixed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionSpec {
    /// Reference to the goal being achieved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal_ref: Option<ObjectReference>,

    /// Reference to the plan to execute.
    pub plan_ref: ObjectReference,

    /// Reference to the binding for capability resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_ref: Option<ObjectReference>,

    /// Reference to the runtime configuration (reserved for future use).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_ref: Option<ObjectReference>,

    /// Parameters to pass to the execution.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, Value>,

    /// Additional context for this execution.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<ContextItem>,
}

/// ExecutionStatus represents the current state of an Execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStatus {
    /// Current phase of the execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<ExecutionPhase>,

    /// Unique identifier for this run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,

    /// When the execution started.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<DateTime<Utc>>,

    /// When the execution completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_time: Option<DateTime<Utc>>,

    /// Status of individual nodes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub node_statuses: BTreeMap<String, NodeStatus>,

    /// Artifacts produced by the execution.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,

    /// The generation most recently observed by the controller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_generation: Option<i64>,

    /// Current conditions of the execution.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
}

/// ExecutionPhase represents the lifecycle phase of an Execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionPhase {
    /// Execution accepted, waiting to start.
    Pending,
    /// Execution in progress.
    Running,
    /// Execution completed successfully.
    Succeeded,
    /// Execution failed.
    Failed,
    /// Execution was cancelled.
    Cancelled,
}

/// NodeStatus represents the status of a single node in an execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeStatus {
    /// Current phase of this node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<NodePhase>,

    /// When this node started.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<DateTime<Utc>>,

    /// When this node completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_time: Option<DateTime<Utc>>,

    /// Current attempt number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt: Option<i32>,

    /// Human-readable message about the node status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// NodePhase represents the lifecycle phase of a node in an execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodePhase {
    /// Node is waiting to start.
    Pending,
    /// Node is currently executing.
    Running,
    /// Node completed successfully.
    Succeeded,
    /// Node failed.
    Failed,
    /// Node was skipped.
    Skipped,
}

/// Artifact represents an output produced by an execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    /// Name of the artifact.
    pub name: String,

    /// Type of the artifact.
    #[serde(rename = "type")]
    pub artifact_type: String,

    /// URI to the artifact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}
