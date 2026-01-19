use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A stored resource object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredObject {
    /// Composite key: namespace/kind/name
    pub namespace: String,
    pub kind: String,
    pub name: String,

    /// The full resource JSON
    pub object: Value,

    /// Server-assigned unique ID
    pub uid: String,

    /// Optimistic concurrency token (increments on any change)
    pub resource_version: i64,

    /// Increments only on spec changes
    pub generation: i64,

    /// Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Watch event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WatchEventType {
    Added,
    Modified,
    Deleted,
}

/// A watch event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEvent {
    #[serde(rename = "type")]
    pub event_type: WatchEventType,
    pub object: Value,
    pub rev: String,
}

/// Resource key for lookups
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ResourceKey {
    pub namespace: String,
    pub kind: String,
    pub name: String,
}

impl ResourceKey {
    pub fn new(namespace: &str, kind: &str, name: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            kind: kind.to_string(),
            name: name.to_string(),
        }
    }
}
