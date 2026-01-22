//! Gate resource type - human-in-the-loop approval checkpoints.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::meta::{Condition, ObjectMeta};
use crate::API_VERSION;

/// Gate represents a human-in-the-loop approval checkpoint that blocks
/// plan execution until explicitly approved or rejected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gate {
    /// API version, always "planspec.io/v1alpha1".
    pub api_version: String,

    /// Kind, always "Gate".
    pub kind: String,

    /// Standard object metadata.
    pub metadata: ObjectMeta,

    /// Specification of the gate.
    pub spec: GateSpec,

    /// Current status of the gate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<GateStatus>,
}

impl Default for Gate {
    fn default() -> Self {
        Self {
            api_version: API_VERSION.to_string(),
            kind: "Gate".to_string(),
            metadata: ObjectMeta::default(),
            spec: GateSpec::default(),
            status: None,
        }
    }
}

impl Gate {
    /// Create a new Gate with the given name and namespace.
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            metadata: ObjectMeta::new(name, namespace),
            ..Default::default()
        }
    }

    /// Set the gate type.
    pub fn with_gate_type(mut self, gate_type: GateType) -> Self {
        self.spec.gate_type = gate_type;
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.spec.description = Some(description.into());
        self
    }

    /// Add a reviewer.
    pub fn with_reviewer(mut self, reviewer: impl Into<String>) -> Self {
        self.spec.reviewers.push(reviewer.into());
        self
    }

    /// Set the number of required approvers.
    pub fn with_required_approvers(mut self, count: i32) -> Self {
        self.spec.required_approvers = Some(count);
        self
    }

    /// Set the timeout.
    pub fn with_timeout(mut self, timeout: impl Into<String>) -> Self {
        self.spec.timeout = Some(timeout.into());
        self
    }
}

/// GateSpec defines the desired state of a Gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateSpec {
    /// Type of approval gate.
    pub gate_type: GateType,

    /// Human-readable description of what this gate approves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// List of authorized reviewers (user IDs, email, or team names).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviewers: Vec<String>,

    /// Number of approvals required (defaults to 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_approvers: Option<i32>,

    /// Maximum time to wait for approval (Go duration format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,

    /// Arbitrary context data to display to reviewers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Default for GateSpec {
    fn default() -> Self {
        Self {
            gate_type: GateType::Approval,
            description: None,
            reviewers: Vec::new(),
            required_approvers: None,
            timeout: None,
            metadata: None,
        }
    }
}

/// GateType represents the type of approval gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GateType {
    /// Simple approval checkpoint.
    Approval,
    /// Code or design review.
    Review,
    /// Formal sign-off (e.g., for compliance).
    SignOff,
}

impl Default for GateType {
    fn default() -> Self {
        Self::Approval
    }
}

/// GateStatus represents the current state of a Gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GateStatus {
    /// Current lifecycle phase.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<GatePhase>,

    /// Current conditions of the gate.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,

    /// History of review actions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub review_history: Vec<ReviewAction>,

    /// Final resolution details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<Resolution>,

    /// The generation most recently observed by the controller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_generation: Option<i64>,
}

/// GatePhase represents the lifecycle phase of a Gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GatePhase {
    /// Gate created but not yet active in any execution.
    Pending,
    /// Execution reached gate node, awaiting reviewer action.
    Waiting,
    /// Reviewer(s) approved, execution can proceed.
    Approved,
    /// Reviewer(s) rejected, execution should fail/halt.
    Rejected,
    /// Reviewer requested changes, may re-review after updates.
    ChangesRequested,
    /// Timeout reached without sufficient approvals.
    Expired,
}

/// ReviewAction represents a single review action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewAction {
    /// Identifier of the reviewer.
    pub reviewer: String,

    /// The action taken.
    pub action: ReviewActionType,

    /// Timestamp of when the action was taken.
    pub timestamp: DateTime<Utc>,

    /// Optional comment explaining the action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// ReviewActionType represents the type of review action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewActionType {
    /// Approve the gate.
    Approve,
    /// Reject the gate.
    Reject,
    /// Request changes before approval.
    RequestChanges,
    /// Comment without taking action.
    Comment,
}

/// Resolution represents the final resolution of a gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    /// Reviewers who approved.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approved_by: Vec<String>,

    /// Reviewers who rejected.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rejected_by: Vec<String>,

    /// Timestamp of final resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}
