//! Context attachment types for embedding notes, code pointers, and structured data.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// ContextItem represents a piece of contextual information attached to a resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextItem {
    /// Optional name for this context item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Format of the content.
    pub format: ContextFormat,

    /// The content itself. Interpretation depends on format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,

    /// List of URIs (for uri-list format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uris: Option<Vec<String>>,
}

impl ContextItem {
    /// Create a markdown context item.
    pub fn markdown(content: impl Into<String>) -> Self {
        Self {
            name: None,
            format: ContextFormat::Markdown,
            content: Some(Value::String(content.into())),
            uris: None,
        }
    }

    /// Create a named markdown context item.
    pub fn markdown_named(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            format: ContextFormat::Markdown,
            content: Some(Value::String(content.into())),
            uris: None,
        }
    }

    /// Create a text context item.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            name: None,
            format: ContextFormat::Text,
            content: Some(Value::String(content.into())),
            uris: None,
        }
    }

    /// Create a JSON context item.
    pub fn json(content: Value) -> Self {
        Self {
            name: None,
            format: ContextFormat::Json,
            content: Some(content),
            uris: None,
        }
    }

    /// Create a URI list context item.
    pub fn uri_list(uris: Vec<String>) -> Self {
        Self {
            name: None,
            format: ContextFormat::UriList,
            content: None,
            uris: Some(uris),
        }
    }

    /// Set the name of this context item.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// ContextFormat specifies how to interpret context content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ContextFormat {
    /// Markdown-formatted text.
    #[default]
    Markdown,
    /// Plain text.
    Text,
    /// Structured JSON data.
    Json,
    /// List of URIs (uses the `uris` field instead of `content`).
    UriList,
}
