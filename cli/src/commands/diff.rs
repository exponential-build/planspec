use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::client::{Client, Config};

/// Show diff between file and server state
pub async fn run(config: &Config, file: &str, output_format: &str) -> Result<()> {
    let path = Path::new(file);
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", file))?;

    let resources = parse_yaml_documents(&content)?;

    if resources.is_empty() {
        println!("{}", "No resources found in file".yellow());
        return Ok(());
    }

    let client = Client::new(config)?;

    for resource in &resources {
        let kind = resource
            .get("kind")
            .and_then(|k| k.as_str())
            .unwrap_or("Unknown");
        let metadata = resource.get("metadata").unwrap_or(&Value::Null);
        let name = metadata.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let default_ns = config.namespace();
        let namespace = metadata
            .get("namespace")
            .and_then(|n| n.as_str())
            .unwrap_or(&default_ns);

        let resource_type = kind_to_resource_type(kind);

        // Try to get the resource from the server
        match client.get(&resource_type, name, Some(namespace)).await {
            Ok(server_resource) => {
                // Resource exists, show diff
                println!(
                    "{} {}/{} ({})",
                    "~".yellow(),
                    kind,
                    name,
                    "would be updated".yellow()
                );

                if output_format == "json" || output_format == "yaml" {
                    // Show full resources
                    println!("--- server");
                    println!("+++ local");
                    show_diff(&server_resource, resource, output_format)?;
                } else {
                    // Show simplified diff
                    show_simple_diff(&server_resource, resource);
                }
            }
            Err(_) => {
                // Resource doesn't exist
                println!(
                    "{} {}/{} ({})",
                    "+".green(),
                    kind,
                    name,
                    "would be created".green()
                );

                if output_format != "table" {
                    println!("{}", serde_yaml::to_string(resource)?);
                }
            }
        }
        println!();
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

fn kind_to_resource_type(kind: &str) -> String {
    match kind {
        "Goal" => "goals",
        "Plan" => "plans",
        "Capability" => "capabilities",
        "Binding" => "bindings",
        "Execution" => "executions",
        other => other,
    }
    .to_string()
}

fn show_diff(server: &Value, local: &Value, format: &str) -> Result<()> {
    // Strip server-only fields for comparison
    let mut server_clean = server.clone();
    let mut local_clean = local.clone();

    // Remove status and system fields for comparison
    if let Some(obj) = server_clean.as_object_mut() {
        obj.remove("status");
        if let Some(meta) = obj.get_mut("metadata").and_then(|m| m.as_object_mut()) {
            meta.remove("uid");
            meta.remove("resourceVersion");
            meta.remove("generation");
            meta.remove("creationTimestamp");
        }
    }

    if let Some(obj) = local_clean.as_object_mut() {
        obj.remove("status");
    }

    let server_str = if format == "json" {
        serde_json::to_string_pretty(&server_clean)?
    } else {
        serde_yaml::to_string(&server_clean)?
    };

    let local_str = if format == "json" {
        serde_json::to_string_pretty(&local_clean)?
    } else {
        serde_yaml::to_string(&local_clean)?
    };

    // Simple line-by-line diff
    let _server_lines: Vec<&str> = server_str.lines().collect();
    let _local_lines: Vec<&str> = local_str.lines().collect();

    for diff in diff::lines(&server_str, &local_str) {
        match diff {
            diff::Result::Left(l) => println!("{}", format!("- {}", l).red()),
            diff::Result::Right(r) => println!("{}", format!("+ {}", r).green()),
            diff::Result::Both(l, _) => println!("  {}", l),
        }
    }

    Ok(())
}

fn show_simple_diff(server: &Value, local: &Value) {
    // Compare spec fields
    let server_spec = server.get("spec").unwrap_or(&Value::Null);
    let local_spec = local.get("spec").unwrap_or(&Value::Null);

    if server_spec != local_spec {
        println!("  {}", "Spec changes:".bold());
        diff_values("spec", server_spec, local_spec, 2);
    }

    // Compare labels
    let server_labels = server
        .get("metadata")
        .and_then(|m| m.get("labels"))
        .unwrap_or(&Value::Null);
    let local_labels = local
        .get("metadata")
        .and_then(|m| m.get("labels"))
        .unwrap_or(&Value::Null);

    if server_labels != local_labels {
        println!("  {}", "Label changes:".bold());
        diff_values("labels", server_labels, local_labels, 2);
    }

    // Compare annotations
    let server_annotations = server
        .get("metadata")
        .and_then(|m| m.get("annotations"))
        .unwrap_or(&Value::Null);
    let local_annotations = local
        .get("metadata")
        .and_then(|m| m.get("annotations"))
        .unwrap_or(&Value::Null);

    if server_annotations != local_annotations {
        println!("  {}", "Annotation changes:".bold());
        diff_values("annotations", server_annotations, local_annotations, 2);
    }
}

fn diff_values(path: &str, server: &Value, local: &Value, indent: usize) {
    let indent_str = " ".repeat(indent);

    match (server, local) {
        (Value::Object(s), Value::Object(l)) => {
            // Find removed keys
            for key in s.keys() {
                if !l.contains_key(key) {
                    println!(
                        "{}{} {}.{}: {}",
                        indent_str,
                        "-".red(),
                        path,
                        key,
                        s.get(key).unwrap()
                    );
                }
            }

            // Find added keys
            for key in l.keys() {
                if !s.contains_key(key) {
                    println!(
                        "{}{} {}.{}: {}",
                        indent_str,
                        "+".green(),
                        path,
                        key,
                        l.get(key).unwrap()
                    );
                }
            }

            // Find changed keys
            for key in s.keys() {
                if let Some(local_val) = l.get(key) {
                    let server_val = s.get(key).unwrap();
                    if server_val != local_val {
                        println!(
                            "{}{} {}.{}: {} -> {}",
                            indent_str,
                            "~".yellow(),
                            path,
                            key,
                            server_val,
                            local_val
                        );
                    }
                }
            }
        }
        _ => {
            if server != local {
                println!("{}{} {}: {} -> {}", indent_str, "~".yellow(), path, server, local);
            }
        }
    }
}

// Simple diff implementation
mod diff {
    pub enum Result<'a> {
        Left(&'a str),
        Right(&'a str),
        Both(&'a str, ()),
    }

    pub fn lines<'a>(old: &'a str, new: &'a str) -> Vec<Result<'a>> {
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();

        let mut results = Vec::new();
        let mut old_idx = 0;
        let mut new_idx = 0;

        while old_idx < old_lines.len() || new_idx < new_lines.len() {
            if old_idx >= old_lines.len() {
                results.push(Result::Right(new_lines[new_idx]));
                new_idx += 1;
            } else if new_idx >= new_lines.len() {
                results.push(Result::Left(old_lines[old_idx]));
                old_idx += 1;
            } else if old_lines[old_idx] == new_lines[new_idx] {
                results.push(Result::Both(old_lines[old_idx], ()));
                old_idx += 1;
                new_idx += 1;
            } else {
                // Simple approach: check if old line appears later in new
                let old_in_new = new_lines[new_idx..].iter().position(|&l| l == old_lines[old_idx]);
                let new_in_old = old_lines[old_idx..].iter().position(|&l| l == new_lines[new_idx]);

                match (old_in_new, new_in_old) {
                    (Some(oin), Some(nio)) if oin <= nio => {
                        // Add new lines until we reach the matching old line
                        for i in 0..oin {
                            results.push(Result::Right(new_lines[new_idx + i]));
                        }
                        new_idx += oin;
                    }
                    (Some(_), Some(_)) => {
                        // Remove old lines until we reach the matching new line
                        results.push(Result::Left(old_lines[old_idx]));
                        old_idx += 1;
                    }
                    (Some(oin), None) => {
                        for i in 0..oin {
                            results.push(Result::Right(new_lines[new_idx + i]));
                        }
                        new_idx += oin;
                    }
                    (None, Some(_)) => {
                        results.push(Result::Left(old_lines[old_idx]));
                        old_idx += 1;
                    }
                    (None, None) => {
                        results.push(Result::Left(old_lines[old_idx]));
                        results.push(Result::Right(new_lines[new_idx]));
                        old_idx += 1;
                        new_idx += 1;
                    }
                }
            }
        }

        results
    }
}
