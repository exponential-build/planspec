//! Goal resource type - high-level objectives with acceptance criteria.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use super::context::ContextItem;
use super::meta::{Condition, ObjectMeta, ObjectReference};
use crate::API_VERSION;

/// Goal represents a high-level objective to be achieved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    /// API version, always "planspec.io/v1alpha1".
    pub api_version: String,

    /// Kind, always "Goal".
    pub kind: String,

    /// Standard object metadata.
    pub metadata: ObjectMeta,

    /// Specification of the desired goal.
    pub spec: GoalSpec,

    /// Current status of the goal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<GoalStatus>,
}

impl Default for Goal {
    fn default() -> Self {
        Self {
            api_version: API_VERSION.to_string(),
            kind: "Goal".to_string(),
            metadata: ObjectMeta::default(),
            spec: GoalSpec::default(),
            status: None,
        }
    }
}

impl Goal {
    /// Create a new Goal with the given name and namespace.
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            metadata: ObjectMeta::new(name, namespace),
            ..Default::default()
        }
    }

    /// Set the description of this goal.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.spec.description = description.into();
        self
    }

    /// Add an acceptance criterion to this goal.
    pub fn with_criterion(mut self, description: impl Into<String>) -> Self {
        self.spec.acceptance_criteria.push(AcceptanceCriterion {
            description: description.into(),
        });
        self
    }

    /// Set the timeout for this goal.
    pub fn with_timeout(mut self, timeout: impl Into<String>) -> Self {
        self.spec.timeout = Some(timeout.into());
        self
    }

    /// Set the priority of this goal.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.spec.priority = Some(priority);
        self
    }
}

/// GoalSpec defines the desired state of a Goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GoalSpec {
    /// Human-readable description of what needs to be achieved.
    pub description: String,

    /// Constraints that apply to achieving this goal.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub constraints: BTreeMap<String, Value>,

    /// Criteria that must be satisfied for the goal to be complete.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acceptance_criteria: Vec<AcceptanceCriterion>,

    /// Label selector for finding plans that satisfy this goal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_selector: Option<LabelSelector>,

    /// Maximum time allowed for achieving this goal (e.g., "7d", "24h").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,

    /// Priority relative to other goals (higher = more important).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,

    /// Additional context for achieving this goal.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<ContextItem>,
}

/// AcceptanceCriterion defines a single criterion for goal completion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptanceCriterion {
    /// Description of what must be true for this criterion to be satisfied.
    pub description: String,
}

/// LabelSelector for matching resources by labels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LabelSelector {
    /// Match labels exactly.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub match_labels: BTreeMap<String, String>,
}

/// GoalStatus represents the current state of a Goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GoalStatus {
    /// Current phase of the goal lifecycle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<GoalPhase>,

    /// Reference to the currently active plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_plan_ref: Option<ObjectReference>,

    /// The generation most recently observed by the controller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_generation: Option<i64>,

    /// Current conditions of the goal.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
}

/// GoalPhase represents the lifecycle phase of a Goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GoalPhase {
    /// Goal created, no plan selected.
    Pending,
    /// Plan is being generated.
    Planning,
    /// Plan exists, ready for execution.
    Ready,
    /// Execution in progress.
    Executing,
    /// All acceptance criteria met.
    Succeeded,
    /// Goal could not be achieved.
    Failed,
    /// Goal was cancelled.
    Cancelled,
}
