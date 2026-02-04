//! Capability resource type - reusable abilities that agents can provide.

use serde::{Deserialize, Serialize};

use super::meta::{Condition, ObjectMeta};
use crate::API_VERSION;

/// Capability represents a reusable ability that can be provided by agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    /// API version, always "planspec.io/v1alpha1".
    pub api_version: String,

    /// Kind, always "Capability".
    pub kind: String,

    /// Standard object metadata.
    pub metadata: ObjectMeta,

    /// Specification of the capability.
    pub spec: CapabilitySpec,

    /// Current status of the capability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<CapabilityStatus>,
}

impl Default for Capability {
    fn default() -> Self {
        Self {
            api_version: API_VERSION.to_string(),
            kind: "Capability".to_string(),
            metadata: ObjectMeta::default(),
            spec: CapabilitySpec::default(),
            status: None,
        }
    }
}

impl Capability {
    /// Create a new Capability with the given name and namespace.
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            metadata: ObjectMeta::new(name, namespace),
            ..Default::default()
        }
    }

    /// Set the description of this capability.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.spec.description = description.into();
        self
    }

    /// Set the display name of this capability.
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.spec.display_name = Some(display_name.into());
        self
    }

    /// Set the category of this capability.
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.spec.category = Some(category.into());
        self
    }
}

/// CapabilitySpec defines the desired state of a Capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitySpec {
    /// Human-friendly name for display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Description of what this capability provides.
    pub description: String,

    /// Category for organizing capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Input parameters this capability accepts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<CapabilityInput>,

    /// Outputs this capability produces.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<CapabilityOutput>,

    /// Other capabilities required by this one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<CapabilityRequirement>,
}

/// CapabilityInput describes an input parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityInput {
    /// Name of the input parameter.
    pub name: String,

    /// Type of the parameter.
    #[serde(rename = "type")]
    pub input_type: CapabilityParamType,

    /// Description of the parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether this input is required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    /// Default value for the parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

/// CapabilityOutput describes an output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityOutput {
    /// Name of the output.
    pub name: String,

    /// Type of the output.
    #[serde(rename = "type")]
    pub output_type: String,
}

/// CapabilityParamType represents the type of a capability parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityParamType {
    String,
    Number,
    Boolean,
    Object,
    Array,
}

/// CapabilityRequirement represents a dependency on another capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRequirement {
    /// Name of the required capability (must match target's metadata.name).
    pub name: String,

    /// Namespace of the required capability (defaults to referring capability's namespace).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// Whether this requirement is optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

/// CapabilityStatus represents the current state of a Capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityStatus {
    /// Current phase of the capability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<CapabilityPhase>,

    /// The generation most recently observed by the controller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_generation: Option<i64>,

    /// Current conditions of the capability.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
}

/// CapabilityPhase represents the lifecycle phase of a Capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CapabilityPhase {
    /// Capability is available for use.
    Available,
    /// Capability is not available.
    Unavailable,
    /// Capability is deprecated.
    Deprecated,
}
