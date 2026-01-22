//! Gate resource type - explicit approval checkpoints.
//!
//! Gates block plan execution until explicitly approved or rejected. While the
//! initial implementation focuses on human reviewers, the data model supports
//! future automation via policy engines or other automated decision systems.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::meta::{Condition, ObjectMeta};
use crate::API_VERSION;

/// Gate represents an explicit approval checkpoint that blocks plan execution
/// until approved or rejected. Reviewers may be humans, policy engines, or
/// other automated systems.
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

    /// Set the target reference.
    pub fn with_target_ref(mut self, target_ref: TargetRef) -> Self {
        self.spec.target_ref = target_ref;
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
}

/// GateSpec defines the desired state of a Gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateSpec {
    /// Type of approval gate.
    pub gate_type: GateType,

    /// Reference to what this gate is gating (required).
    pub target_ref: TargetRef,

    /// Human-readable description of what this gate approves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// List of authorized reviewers (user IDs, emails, team names, or service account identifiers).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviewers: Vec<String>,

    /// Number of distinct approvals required (defaults to 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_approvers: Option<i32>,

    /// Arbitrary context data to display to reviewers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

impl Default for GateSpec {
    fn default() -> Self {
        Self {
            gate_type: GateType::Approval,
            target_ref: TargetRef::default(),
            description: None,
            reviewers: Vec::new(),
            required_approvers: None,
            context: None,
        }
    }
}

/// TargetRef identifies what this gate is gating. Follows K8s ObjectReference conventions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TargetRef {
    /// API version of the target resource (e.g., "planspec.io/v1alpha1").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,

    /// Kind of the target resource (e.g., "Execution", "Plan", "Artifact").
    pub kind: String,

    /// Name of the target resource.
    pub name: String,

    /// Namespace of the target resource (defaults to gate's namespace).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// UID of the target resource for strong references (survives renames).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,

    /// ID of a specific node within the target Plan/Execution graph.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
}

impl TargetRef {
    /// Create a new TargetRef for a resource.
    pub fn new(kind: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            api_version: None,
            kind: kind.into(),
            name: name.into(),
            namespace: None,
            uid: None,
            node_id: None,
        }
    }

    /// Create a new TargetRef for an Execution.
    pub fn execution(name: impl Into<String>) -> Self {
        Self::new("Execution", name)
    }

    /// Create a new TargetRef for a Plan.
    pub fn plan(name: impl Into<String>) -> Self {
        Self::new("Plan", name)
    }

    /// Set the API version.
    pub fn with_api_version(mut self, api_version: impl Into<String>) -> Self {
        self.api_version = Some(api_version.into());
        self
    }

    /// Set the namespace.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Set the UID.
    pub fn with_uid(mut self, uid: impl Into<String>) -> Self {
        self.uid = Some(uid.into());
        self
    }

    /// Set the node ID.
    pub fn with_node_id(mut self, node_id: impl Into<String>) -> Self {
        self.node_id = Some(node_id.into());
        self
    }
}

/// GateType represents the type of approval gate.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GateType {
    /// Simple approval checkpoint.
    #[default]
    Approval,
    /// Code or design review.
    Review,
    /// Formal sign-off (e.g., for compliance).
    SignOff,
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

    /// History of review actions for audit trail.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub review_history: Vec<ReviewAction>,

    /// Final resolution details (required when phase is terminal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<Resolution>,

    /// The spec generation observed by the controller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_generation: Option<i64>,

    /// The spec generation the final resolution applies to (required when resolution exists).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decided_generation: Option<i64>,
}

/// GatePhase represents the lifecycle phase of a Gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GatePhase {
    /// Gate created but not yet activated by controller.
    Pending,
    /// Active, awaiting resolution.
    Waiting,
    /// Approved, execution can proceed.
    Approved,
    /// Rejected, execution should fail/halt.
    Rejected,
}

/// ReviewAction represents a single review action in the audit trail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewAction {
    /// Identifier of the reviewer (user, team, or service account).
    pub reviewer: String,

    /// The action taken.
    pub action: ReviewActionType,

    /// Timestamp of when the action was taken.
    pub timestamp: DateTime<Utc>,

    /// Optional comment explaining the action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    /// The spec generation this action was taken against (for detecting stale approvals).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_generation: Option<i64>,
}

/// ReviewActionType represents the type of review action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewActionType {
    /// Approve the gate.
    Approve,
    /// Reject the gate.
    Reject,
    /// Comment without taking action.
    Comment,
}

/// Resolution represents the final resolution of a gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    /// The final outcome.
    pub outcome: ResolutionOutcome,

    /// Identifiers of reviewers who contributed to this outcome.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actors: Vec<String>,

    /// Timestamp of final resolution.
    pub timestamp: DateTime<Utc>,

    /// Optional summary comment for the resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// ResolutionOutcome represents the final outcome of a gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResolutionOutcome {
    /// Gate was approved.
    Approved,
    /// Gate was rejected.
    Rejected,
}
