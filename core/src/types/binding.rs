//! Binding resource type - resolution of capabilities to provider targets.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::meta::{Condition, ObjectMeta, ObjectReference};
use crate::API_VERSION;

/// Binding resolves capability requirements to provider targets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    /// API version, always "planspec.io/v1alpha1".
    pub api_version: String,

    /// Kind, always "Binding".
    pub kind: String,

    /// Standard object metadata.
    pub metadata: ObjectMeta,

    /// Specification of the binding.
    pub spec: BindingSpec,

    /// Current status of the binding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BindingStatus>,
}

impl Default for Binding {
    fn default() -> Self {
        Self {
            api_version: API_VERSION.to_string(),
            kind: "Binding".to_string(),
            metadata: ObjectMeta::default(),
            spec: BindingSpec::default(),
            status: None,
        }
    }
}

impl Binding {
    /// Create a new Binding with the given name and namespace.
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            metadata: ObjectMeta::new(name, namespace),
            ..Default::default()
        }
    }

    /// Add a rule to this binding.
    pub fn with_rule(mut self, rule: BindingRule) -> Self {
        self.spec.rules.push(rule);
        self
    }
}

/// BindingSpec defines the desired state of a Binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BindingSpec {
    /// Rules for resolving capabilities to providers.
    pub rules: Vec<BindingRule>,
}

/// BindingRule maps selectors to provider targets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingRule {
    /// Selector for matching nodes/capabilities.
    pub selector: BindingSelector,

    /// Target provider for matched nodes.
    pub target: BindingTarget,
}

impl BindingRule {
    /// Create a rule that binds a capability to a provider.
    pub fn for_capability(capability_name: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            selector: BindingSelector {
                capability_ref: Some(ObjectReference::new(capability_name)),
                plan_ref: None,
                node_id: None,
            },
            target: BindingTarget {
                provider: provider.into(),
                config: None,
            },
        }
    }

    /// Create a rule that binds a specific node in a plan.
    pub fn for_node(
        plan_name: impl Into<String>,
        node_id: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        Self {
            selector: BindingSelector {
                capability_ref: None,
                plan_ref: Some(ObjectReference::new(plan_name)),
                node_id: Some(node_id.into()),
            },
            target: BindingTarget {
                provider: provider.into(),
                config: None,
            },
        }
    }

    /// Add configuration to this rule's target.
    pub fn with_config(mut self, config: Value) -> Self {
        self.target.config = Some(config);
        self
    }
}

/// BindingSelector matches nodes for binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BindingSelector {
    /// Match by capability reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_ref: Option<ObjectReference>,

    /// Match by plan reference (used with node_id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_ref: Option<ObjectReference>,

    /// Match by specific node ID (used with plan_ref).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
}

/// BindingTarget specifies the provider for matched nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingTarget {
    /// Provider URI (e.g., "agent://claude-code").
    pub provider: String,

    /// Provider-specific configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<Value>,
}

/// BindingStatus represents the current state of a Binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BindingStatus {
    /// Current phase of the binding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<BindingPhase>,

    /// Number of capabilities successfully bound.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bound_count: Option<i64>,

    /// Capabilities that could not be bound.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unbound_capabilities: Vec<String>,

    /// The generation most recently observed by the controller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_generation: Option<i64>,

    /// Current conditions of the binding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
}

/// BindingPhase represents the lifecycle phase of a Binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BindingPhase {
    /// All capabilities are bound.
    Ready,
    /// Some capabilities are bound.
    PartiallyBound,
    /// No capabilities could be bound.
    Unresolved,
}
