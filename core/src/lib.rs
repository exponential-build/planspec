//! # PlanSpec Core
//!
//! Core types and validation for the PlanSpec declarative work orchestration system.
//!
//! ## Overview
//!
//! This crate provides the foundational types for PlanSpec resources:
//! - [`Goal`] - High-level objectives with acceptance criteria
//! - [`Plan`] - DAGs of work with embedded nodes and edges
//! - [`Capability`] - Reusable abilities that agents can provide
//! - [`Binding`] - Resolution of capabilities to provider targets
//! - [`Execution`] - Intent to run a Plan with specific bindings
//!
//! ## Example
//!
//! ```rust
//! use planspec_core::{Goal, GoalSpec, ObjectMeta};
//!
//! let goal = Goal {
//!     api_version: "planspec.io/v1alpha1".to_string(),
//!     kind: "Goal".to_string(),
//!     metadata: ObjectMeta::new("my-goal", "default"),
//!     spec: GoalSpec {
//!         description: "Implement feature X".to_string(),
//!         ..Default::default()
//!     },
//!     status: None,
//! };
//! ```

mod resource;
pub mod types;
pub mod validate;

#[cfg(test)]
mod tests;

// Re-export commonly used types at crate root
pub use resource::Resource;
pub use types::api::{
    AppliedResource, ApplyAction, ApplyError, ApplyResult, ListMeta, ResourceList, Status,
    StatusCause, StatusDetails, StatusReason, StatusResult, WatchEvent, WatchEventType,
};
pub use types::binding::{
    Binding, BindingPhase, BindingRule, BindingSelector, BindingSpec, BindingStatus, BindingTarget,
};
pub use types::capability::{
    Capability, CapabilityInput, CapabilityOutput, CapabilityParamType, CapabilityPhase,
    CapabilityRequirement, CapabilitySpec, CapabilityStatus,
};
pub use types::context::{ContextFormat, ContextItem};
pub use types::execution::{
    Artifact, Execution, ExecutionPhase, ExecutionSpec, ExecutionStatus, NodePhase, NodeStatus,
};
pub use types::gate::{
    Gate, GatePhase, GateSpec, GateStatus, GateType, Resolution, ReviewAction, ReviewActionType,
};
pub use types::goal::{AcceptanceCriterion, Goal, GoalPhase, GoalSpec, GoalStatus, LabelSelector};
pub use types::meta::{Condition, ConditionStatus, ObjectMeta, ObjectReference, OwnerReference};
pub use types::plan::{
    AcceptanceCriteria, Edge, EdgeType, EstimatedEffort, ExpectedResponse, Graph, GraphError, Node,
    NodeKind, Plan, PlanPhase, PlanSpec, PlanStatus,
};
pub use validate::{ValidationError, Validator};

/// The API version for all PlanSpec resources
pub const API_VERSION: &str = "planspec.io/v1alpha1";
