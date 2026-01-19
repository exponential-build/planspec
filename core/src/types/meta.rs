//! Common metadata types shared by all PlanSpec resources.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// ObjectMeta contains metadata that all persisted resources must have.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ObjectMeta {
    /// Name must be unique within a namespace.
    pub name: String,

    /// Namespace defines the space within which each name must be unique.
    pub namespace: String,

    /// UID is the unique identifier for this object, set by the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,

    /// ResourceVersion is used for optimistic concurrency control.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<String>,

    /// Generation is incremented by the server when spec changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<i64>,

    /// CreationTimestamp is when the object was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_timestamp: Option<DateTime<Utc>>,

    /// Labels are key-value pairs for organizing and selecting resources.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,

    /// Annotations are key-value pairs for storing arbitrary metadata.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,

    /// OwnerReferences lists objects that own this object.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owner_references: Vec<OwnerReference>,
}

impl ObjectMeta {
    /// Create a new ObjectMeta with name and namespace.
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: namespace.into(),
            ..Default::default()
        }
    }

    /// Add a label to this object.
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Add an annotation to this object.
    pub fn with_annotation(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.annotations.insert(key.into(), value.into());
        self
    }

    /// Add an owner reference to this object.
    pub fn with_owner(mut self, owner: OwnerReference) -> Self {
        self.owner_references.push(owner);
        self
    }
}

/// ObjectReference contains information to locate a referenced object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ObjectReference {
    /// Name of the referent.
    pub name: String,

    /// Namespace of the referent. Defaults to the referring object's namespace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// UID of the referent for strong references.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
}

impl ObjectReference {
    /// Create a new ObjectReference with just a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: None,
            uid: None,
        }
    }

    /// Create a new ObjectReference with name and namespace.
    pub fn with_namespace(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: Some(namespace.into()),
            uid: None,
        }
    }
}

/// OwnerReference contains information to let you identify an owning object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerReference {
    /// API version of the referent.
    pub api_version: String,

    /// Kind of the referent.
    pub kind: String,

    /// Name of the referent.
    pub name: String,

    /// UID of the referent.
    pub uid: String,

    /// If true, this reference points to the managing controller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller: Option<bool>,

    /// If true, block owner deletion until this object is deleted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_owner_deletion: Option<bool>,
}

/// Condition represents the state of a resource at a point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    /// Type of condition (e.g., "Ready", "Progressing").
    #[serde(rename = "type")]
    pub condition_type: String,

    /// Status of the condition: True, False, or Unknown.
    pub status: ConditionStatus,

    /// Machine-readable reason for the condition's last transition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Human-readable message with details about the transition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// When the condition last transitioned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_transition_time: Option<DateTime<Utc>>,

    /// Generation observed when the condition was set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_generation: Option<i64>,
}

/// ConditionStatus represents the status of a condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionStatus {
    /// Condition is true.
    True,
    /// Condition is false.
    False,
    /// Condition status is unknown.
    Unknown,
}

impl Default for ConditionStatus {
    fn default() -> Self {
        Self::Unknown
    }
}
