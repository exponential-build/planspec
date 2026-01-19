use anyhow::{Context, Result};
use colored::Colorize;
use planspec_core::Validator;
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::client::{Client, Config};

/// Apply resources from a file
pub async fn run(config: &Config, file: &str, dry_run: bool, output_format: &str) -> Result<()> {
    let path = Path::new(file);
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", file))?;

    // Parse YAML (supports multi-document)
    let resources = parse_yaml_documents(&content)?;

    if resources.is_empty() {
        println!("{}", "No resources found in file".yellow());
        return Ok(());
    }

    // Validate locally first
    let validator = Validator::new()
        .map_err(|e| anyhow::anyhow!("Failed to create validator: {}", e))?;
    for resource in &resources {
        if let Err(errors) = validator.validate_json(resource) {
            let kind = resource.get("kind").and_then(|k| k.as_str()).unwrap_or("Unknown");
            let name = resource
                .get("metadata")
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("<unnamed>");

            eprintln!("{} {}/{} has validation errors:", "✗".red(), kind, name);
            for error in errors {
                eprintln!("  {} {}", "→".yellow(), error);
            }
            anyhow::bail!("Local validation failed");
        }
    }

    if dry_run {
        println!("{}", "Dry run mode - validation passed".green());
        for resource in &resources {
            let kind = resource.get("kind").and_then(|k| k.as_str()).unwrap_or("Unknown");
            let name = resource
                .get("metadata")
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("<unnamed>");
            println!("  {} {}/{} would be applied", "→".blue(), kind, name);
        }
        return Ok(());
    }

    // Group resources by namespace from their metadata
    let client = Client::new(config)?;
    let default_ns = config.namespace();

    // Group resources by their metadata.namespace
    let mut by_namespace: std::collections::HashMap<String, Vec<Value>> = std::collections::HashMap::new();
    for resource in resources {
        let ns = resource
            .get("metadata")
            .and_then(|m| m.get("namespace"))
            .and_then(|n| n.as_str())
            .unwrap_or(&default_ns)
            .to_string();
        by_namespace.entry(ns).or_default().push(resource);
    }

    // Apply each namespace group
    let mut all_applied = Vec::new();
    let mut all_errors = Vec::new();

    for (namespace, ns_resources) in by_namespace {
        let result = client.apply(&namespace, &ns_resources).await?;
        if let Some(applied) = result.get("applied").and_then(|a| a.as_array()) {
            all_applied.extend(applied.clone());
        }
        if let Some(errors) = result.get("errors").and_then(|e| e.as_array()) {
            all_errors.extend(errors.clone());
        }
    }

    let result = serde_json::json!({
        "applied": all_applied,
        "errors": all_errors
    });

    match output_format {
        "json" => println!("{}", serde_json::to_string_pretty(&result)?),
        "yaml" => println!("{}", serde_yaml::to_string(&result)?),
        _ => output_apply_result(&result),
    }

    Ok(())
}

fn parse_yaml_documents(content: &str) -> Result<Vec<Value>> {
    let mut resources = Vec::new();

    for doc in serde_yaml::Deserializer::from_str(content) {
        let value: Value = serde::Deserialize::deserialize(doc)
            .context("Failed to parse YAML document")?;

        if value.is_null() {
            continue;
        }

        if let Some(kind) = value.get("kind").and_then(|k| k.as_str()) {
            if kind == "List" {
                if let Some(items) = value.get("items").and_then(|i| i.as_array()) {
                    resources.extend(items.clone());
                    continue;
                }
            }
        }

        resources.push(value);
    }

    Ok(resources)
}

fn output_apply_result(result: &Value) {
    if let Some(applied) = result.get("applied").and_then(|a| a.as_array()) {
        for item in applied {
            let kind = item.get("kind").and_then(|k| k.as_str()).unwrap_or("");
            let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let ns = item.get("namespace").and_then(|n| n.as_str()).unwrap_or("");
            let action = item.get("action").and_then(|a| a.as_str()).unwrap_or("");

            let action_colored = match action {
                "created" => "created".green(),
                "updated" => "configured".yellow(),
                "unchanged" => "unchanged".blue(),
                _ => action.normal(),
            };

            println!("{}/{}/{} {}", ns, kind.to_lowercase(), name, action_colored);
        }
    }

    if let Some(errors) = result.get("errors").and_then(|e| e.as_array()) {
        for error in errors {
            let resource = error.get("resource").and_then(|r| r.as_str()).unwrap_or("");
            let message = error.get("message").and_then(|m| m.as_str()).unwrap_or("");
            eprintln!("{} {}: {}", "✗".red(), resource, message);
        }
    }
}
