//! API types for client/server interoperability.
//!
//! These types are used in API responses and watch streams.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::API_VERSION;

/// Watch event types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WatchEventType {
    /// Resource was added.
    Added,
    /// Resource was modified.
    Modified,
    /// Resource was deleted.
    Deleted,
}

/// A watch event from the API server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchEvent {
    /// Type of event.
    #[serde(rename = "type")]
    pub event_type: WatchEventType,

    /// The resource object.
    pub object: Value,

    /// Server-assigned revision for resuming watches.
    pub rev: String,
}

impl WatchEvent {
    /// Create a new watch event.
    pub fn new(event_type: WatchEventType, object: Value, rev: impl Into<String>) -> Self {
        Self {
            event_type,
            object,
            rev: rev.into(),
        }
    }

    /// Create an ADDED event.
    pub fn added(object: Value, rev: impl Into<String>) -> Self {
        Self::new(WatchEventType::Added, object, rev)
    }

    /// Create a MODIFIED event.
    pub fn modified(object: Value, rev: impl Into<String>) -> Self {
        Self::new(WatchEventType::Modified, object, rev)
    }

    /// Create a DELETED event.
    pub fn deleted(object: Value, rev: impl Into<String>) -> Self {
        Self::new(WatchEventType::Deleted, object, rev)
    }
}

/// K8s-style Status response for API errors and success messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// API version.
    pub api_version: String,

    /// Always "Status".
    pub kind: String,

    /// "Success" or "Failure".
    pub status: StatusResult,

    /// Human-readable message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Machine-readable reason (e.g., "NotFound", "Invalid", "Conflict").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<StatusReason>,

    /// Additional details about the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<StatusDetails>,

    /// HTTP status code.
    pub code: u16,
}

impl Status {
    /// Create a success status.
    pub fn success(message: impl Into<String>, code: u16) -> Self {
        Self {
            api_version: API_VERSION.to_string(),
            kind: "Status".to_string(),
            status: StatusResult::Success,
            message: Some(message.into()),
            reason: None,
            details: None,
            code,
        }
    }

    /// Create a failure status.
    pub fn failure(message: impl Into<String>, reason: StatusReason, code: u16) -> Self {
        Self {
            api_version: API_VERSION.to_string(),
            kind: "Status".to_string(),
            status: StatusResult::Failure,
            message: Some(message.into()),
            reason: Some(reason),
            details: None,
            code,
        }
    }

    /// Create a not found status.
    pub fn not_found(resource: &str, name: &str) -> Self {
        Self::failure(
            format!("{} '{}' not found", resource, name),
            StatusReason::NotFound,
            404,
        )
    }

    /// Create an invalid status.
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::failure(message, StatusReason::Invalid, 400)
    }

    /// Create a conflict status.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::failure(message, StatusReason::Conflict, 409)
    }

    /// Create an already exists status.
    pub fn already_exists(resource: &str, name: &str) -> Self {
        Self::failure(
            format!("{} '{}' already exists", resource, name),
            StatusReason::AlreadyExists,
            409,
        )
    }

    /// Create an internal error status.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::failure(message, StatusReason::InternalError, 500)
    }

    /// Add details to this status.
    pub fn with_details(mut self, details: StatusDetails) -> Self {
        self.details = Some(details);
        self
    }
}

/// Result of an API operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatusResult {
    /// Operation succeeded.
    Success,
    /// Operation failed.
    Failure,
}

/// Machine-readable reason for a status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatusReason {
    /// Resource not found.
    NotFound,
    /// Resource already exists.
    AlreadyExists,
    /// Resource validation failed.
    Invalid,
    /// Optimistic concurrency conflict.
    Conflict,
    /// Permission denied.
    Forbidden,
    /// Authentication required.
    Unauthorized,
    /// Server error.
    InternalError,
    /// Service unavailable.
    ServiceUnavailable,
    /// Request timeout.
    Timeout,
}

/// Additional details about a status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StatusDetails {
    /// Name of the resource (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Kind of the resource (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,

    /// Causes of the error.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causes: Vec<StatusCause>,

    /// Retry after seconds (for rate limiting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<i64>,
}

/// A specific cause of a status failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusCause {
    /// Resource identifier (e.g., "plans/my-plan").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,

    /// Machine-readable reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Human-readable message.
    pub message: String,

    /// JSON path to the field that caused the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl StatusCause {
    /// Create a new status cause.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            resource: None,
            reason: None,
            message: message.into(),
            field: None,
        }
    }

    /// Set the resource for this cause.
    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Set the reason for this cause.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Set the field for this cause.
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

/// A list of resources returned by list operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceList<T> {
    /// API version.
    pub api_version: String,

    /// Kind (e.g., "GoalList", "PlanList").
    pub kind: String,

    /// The resources.
    pub items: Vec<T>,

    /// Metadata about the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ListMeta>,
}

impl<T> ResourceList<T> {
    /// Create a new resource list.
    pub fn new(kind: impl Into<String>, items: Vec<T>) -> Self {
        Self {
            api_version: API_VERSION.to_string(),
            kind: kind.into(),
            items,
            metadata: None,
        }
    }

    /// Create an empty list.
    pub fn empty(kind: impl Into<String>) -> Self {
        Self::new(kind, Vec::new())
    }

    /// Set the list metadata.
    pub fn with_metadata(mut self, metadata: ListMeta) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Metadata about a list response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListMeta {
    /// Resource version for the list (for watch resumption).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<String>,

    /// Token for retrieving the next page (if paginated).
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_token: Option<String>,

    /// Remaining item count (if paginated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_item_count: Option<i64>,
}

/// Result of a bulk apply operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    /// Resources that were successfully applied.
    pub applied: Vec<AppliedResource>,

    /// Errors that occurred during apply.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ApplyError>,
}

impl ApplyResult {
    /// Create an empty apply result.
    pub fn new() -> Self {
        Self {
            applied: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Check if all resources were applied successfully.
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Default for ApplyResult {
    fn default() -> Self {
        Self::new()
    }
}

/// A successfully applied resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedResource {
    /// Kind of the resource.
    pub kind: String,

    /// Name of the resource.
    pub name: String,

    /// Namespace of the resource.
    pub namespace: String,

    /// Action taken.
    pub action: ApplyAction,
}

/// Action taken when applying a resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApplyAction {
    /// Resource was created.
    Created,
    /// Resource was updated.
    Updated,
    /// Resource was unchanged.
    Unchanged,
}

/// An error that occurred during apply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyError {
    /// Resource identifier (e.g., "Goal/my-goal").
    pub resource: String,

    /// Error message.
    pub message: String,
}
