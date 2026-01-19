//! Polymorphic resource handling.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::binding::Binding;
use crate::types::capability::Capability;
use crate::types::execution::Execution;
use crate::types::goal::Goal;
use crate::types::meta::ObjectMeta;
use crate::types::plan::Plan;

/// Resource is an enum that can hold any PlanSpec resource type.
///
/// Uses untagged serialization since each resource type has its own `kind` field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Resource {
    /// A Goal resource.
    Goal(Goal),
    /// A Plan resource.
    Plan(Plan),
    /// A Capability resource.
    Capability(Capability),
    /// A Binding resource.
    Binding(Binding),
    /// An Execution resource.
    Execution(Execution),
}

impl Resource {
    /// Get the kind of this resource.
    pub fn kind(&self) -> &str {
        match self {
            Resource::Goal(_) => "Goal",
            Resource::Plan(_) => "Plan",
            Resource::Capability(_) => "Capability",
            Resource::Binding(_) => "Binding",
            Resource::Execution(_) => "Execution",
        }
    }

    /// Get the metadata of this resource.
    pub fn metadata(&self) -> &ObjectMeta {
        match self {
            Resource::Goal(r) => &r.metadata,
            Resource::Plan(r) => &r.metadata,
            Resource::Capability(r) => &r.metadata,
            Resource::Binding(r) => &r.metadata,
            Resource::Execution(r) => &r.metadata,
        }
    }

    /// Get a mutable reference to the metadata of this resource.
    pub fn metadata_mut(&mut self) -> &mut ObjectMeta {
        match self {
            Resource::Goal(r) => &mut r.metadata,
            Resource::Plan(r) => &mut r.metadata,
            Resource::Capability(r) => &mut r.metadata,
            Resource::Binding(r) => &mut r.metadata,
            Resource::Execution(r) => &mut r.metadata,
        }
    }

    /// Get the name of this resource.
    pub fn name(&self) -> &str {
        &self.metadata().name
    }

    /// Get the namespace of this resource.
    pub fn namespace(&self) -> &str {
        &self.metadata().namespace
    }

    /// Get the API version of this resource.
    pub fn api_version(&self) -> &str {
        match self {
            Resource::Goal(r) => &r.api_version,
            Resource::Plan(r) => &r.api_version,
            Resource::Capability(r) => &r.api_version,
            Resource::Binding(r) => &r.api_version,
            Resource::Execution(r) => &r.api_version,
        }
    }
}

impl From<Goal> for Resource {
    fn from(goal: Goal) -> Self {
        Resource::Goal(goal)
    }
}

impl From<Plan> for Resource {
    fn from(plan: Plan) -> Self {
        Resource::Plan(plan)
    }
}

impl From<Capability> for Resource {
    fn from(capability: Capability) -> Self {
        Resource::Capability(capability)
    }
}

impl From<Binding> for Resource {
    fn from(binding: Binding) -> Self {
        Resource::Binding(binding)
    }
}

impl From<Execution> for Resource {
    fn from(execution: Execution) -> Self {
        Resource::Execution(execution)
    }
}

impl TryFrom<Value> for Resource {
    type Error = ResourceParseError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let kind = value
            .get("kind")
            .and_then(|k| k.as_str())
            .ok_or(ResourceParseError::MissingKind)?;

        match kind {
            "Goal" => {
                let goal: Goal = serde_json::from_value(value)
                    .map_err(|e| ResourceParseError::DeserializationError(e.to_string()))?;
                Ok(Resource::Goal(goal))
            }
            "Plan" => {
                let plan: Plan = serde_json::from_value(value)
                    .map_err(|e| ResourceParseError::DeserializationError(e.to_string()))?;
                Ok(Resource::Plan(plan))
            }
            "Capability" => {
                let capability: Capability = serde_json::from_value(value)
                    .map_err(|e| ResourceParseError::DeserializationError(e.to_string()))?;
                Ok(Resource::Capability(capability))
            }
            "Binding" => {
                let binding: Binding = serde_json::from_value(value)
                    .map_err(|e| ResourceParseError::DeserializationError(e.to_string()))?;
                Ok(Resource::Binding(binding))
            }
            "Execution" => {
                let execution: Execution = serde_json::from_value(value)
                    .map_err(|e| ResourceParseError::DeserializationError(e.to_string()))?;
                Ok(Resource::Execution(execution))
            }
            _ => Err(ResourceParseError::UnknownKind(kind.to_string())),
        }
    }
}

/// Error parsing a resource from JSON.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ResourceParseError {
    /// The JSON is missing the "kind" field.
    #[error("missing 'kind' field")]
    MissingKind,

    /// The kind is not recognized.
    #[error("unknown resource kind: {0}")]
    UnknownKind(String),

    /// Failed to deserialize the resource.
    #[error("deserialization error: {0}")]
    DeserializationError(String),
}
