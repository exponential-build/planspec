use serde::Serialize;
use serde_json::Value;

/// YAML output formatter
#[allow(dead_code)]
pub struct YamlFormatter;

impl YamlFormatter {
    /// Format a value as YAML
    pub fn format<T: Serialize>(value: &T) -> String {
        serde_yaml::to_string(value).unwrap_or_else(|e| format!("Error formatting YAML: {}", e))
    }

    /// Format a JSON value as YAML
    pub fn format_value(value: &Value) -> String {
        serde_yaml::to_string(value).unwrap_or_else(|e| format!("Error formatting YAML: {}", e))
    }

    /// Format multiple resources as multi-document YAML
    pub fn format_multi(values: &[Value]) -> String {
        values
            .iter()
            .map(|v| serde_yaml::to_string(v).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("---\n")
    }
}
