use anyhow::{Context, Result};
use colored::Colorize;
use planspec_core::Validator;
use serde_json::Value;
use std::fs;

use crate::client::{Client, Config};
use crate::commands::utils::InputSource;

/// Apply resources from a file or directory
pub async fn run(
    config: &Config,
    file: Option<String>,
    directory: Option<String>,
    recursive: bool,
    dry_run: bool,
    output_format: &str,
) -> Result<()> {
    let source = InputSource::from_args(file, directory, recursive)?;
    let files = source.get_files()?;

    if files.is_empty() {
        println!("{}", "No YAML files found".yellow());
        return Ok(());
    }

    // Track results across all files
    let mut total_applied = Vec::new();
    let mut total_errors = Vec::new();
    let mut file_errors: Vec<(String, String)> = Vec::new();

    let validator =
        Validator::new().map_err(|e| anyhow::anyhow!("Failed to create validator: {}", e))?;
    let client = if !dry_run {
        Some(Client::new(config)?)
    } else {
        None
    };
    let default_ns = config.namespace();

    for file_path in &files {
        let file_display = file_path.display().to_string();

        // Read and parse file
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                file_errors.push((file_display, format!("Failed to read: {}", e)));
                continue;
            }
        };

        let resources = match parse_yaml_documents(&content) {
            Ok(r) => r,
            Err(e) => {
                file_errors.push((file_display, format!("Failed to parse YAML: {}", e)));
                continue;
            }
        };

        if resources.is_empty() {
            continue;
        }

        // Validate locally
        let mut validation_failed = false;
        for resource in &resources {
            if let Err(errors) = validator.validate_json(resource) {
                validation_failed = true;
                let kind = resource
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .unwrap_or("Unknown");
                let name = resource
                    .get("metadata")
                    .and_then(|m| m.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("<unnamed>");

                let error_msg = errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                file_errors.push((
                    file_display.clone(),
                    format!("{}/{}: {}", kind, name, error_msg),
                ));
            }
        }

        if validation_failed {
            continue;
        }

        if dry_run {
            // Just report what would be applied
            for resource in &resources {
                let kind = resource
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .unwrap_or("Unknown");
                let name = resource
                    .get("metadata")
                    .and_then(|m| m.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("<unnamed>");
                total_applied.push(serde_json::json!({
                    "kind": kind,
                    "name": name,
                    "action": "would-apply",
                    "file": file_display
                }));
            }
        } else {
            // Apply to server
            let client = client.as_ref().unwrap();

            // Group resources by namespace
            let mut by_namespace: std::collections::HashMap<String, Vec<Value>> =
                std::collections::HashMap::new();
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
            for (namespace, ns_resources) in by_namespace {
                match client.apply(&namespace, &ns_resources).await {
                    Ok(result) => {
                        if let Some(applied) = result.get("applied").and_then(|a| a.as_array()) {
                            for item in applied {
                                let mut item = item.clone();
                                if let Some(obj) = item.as_object_mut() {
                                    obj.insert(
                                        "file".to_string(),
                                        serde_json::Value::String(file_display.clone()),
                                    );
                                }
                                total_applied.push(item);
                            }
                        }
                        if let Some(errors) = result.get("errors").and_then(|e| e.as_array()) {
                            total_errors.extend(errors.clone());
                        }
                    }
                    Err(e) => {
                        file_errors.push((file_display.clone(), format!("Apply failed: {}", e)));
                    }
                }
            }
        }
    }

    // Output results
    let result = serde_json::json!({
        "applied": total_applied,
        "errors": total_errors,
        "fileErrors": file_errors.iter().map(|(f, e)| serde_json::json!({"file": f, "error": e})).collect::<Vec<_>>()
    });

    match output_format {
        "json" => println!("{}", serde_json::to_string_pretty(&result)?),
        "yaml" => println!("{}", serde_yaml::to_string(&result)?),
        _ => output_apply_result(&result, dry_run, files.len()),
    }

    // Return error if any files had errors
    if !file_errors.is_empty() || !total_errors.is_empty() {
        let total_error_count = file_errors.len() + total_errors.len();
        anyhow::bail!(
            "{} error(s) occurred during apply",
            total_error_count
        );
    }

    Ok(())
}

fn parse_yaml_documents(content: &str) -> Result<Vec<Value>> {
    let mut resources = Vec::new();

    for doc in serde_yaml::Deserializer::from_str(content) {
        let value: Value =
            serde::Deserialize::deserialize(doc).context("Failed to parse YAML document")?;

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

fn output_apply_result(result: &Value, dry_run: bool, file_count: usize) {
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
                "would-apply" => "would be applied".cyan(),
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

    if let Some(file_errors) = result.get("fileErrors").and_then(|e| e.as_array()) {
        for error in file_errors {
            let file = error.get("file").and_then(|f| f.as_str()).unwrap_or("");
            let message = error.get("error").and_then(|m| m.as_str()).unwrap_or("");
            eprintln!("{} {}: {}", "✗".red(), file, message);
        }
    }

    // Summary
    let applied_count = result
        .get("applied")
        .and_then(|a| a.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let error_count = result
        .get("errors")
        .and_then(|e| e.as_array())
        .map(|e| e.len())
        .unwrap_or(0)
        + result
            .get("fileErrors")
            .and_then(|e| e.as_array())
            .map(|e| e.len())
            .unwrap_or(0);

    if file_count > 1 {
        println!();
        if dry_run {
            println!(
                "Dry run: {} resource(s) from {} file(s) would be applied",
                applied_count, file_count
            );
        } else {
            print!(
                "{} resource(s) applied from {} file(s)",
                applied_count, file_count
            );
            if error_count > 0 {
                print!(", {} error(s)", error_count);
            }
            println!();
        }
    }
}
