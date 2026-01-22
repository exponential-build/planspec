use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::fs;

use crate::client::{Client, Config};
use crate::commands::utils::InputSource;

/// Normalize singular resource type to plural for API
fn normalize_resource_type(resource: &str) -> &str {
    match resource.to_lowercase().as_str() {
        "goal" => "goals",
        "plan" => "plans",
        "capability" => "capabilities",
        "binding" => "bindings",
        "execution" => "executions",
        // Already plural or unknown - pass through as-is
        _ => resource,
    }
}

/// Check if resource type is a namespace
fn is_namespace(resource: &str) -> bool {
    matches!(
        resource.to_lowercase().as_str(),
        "namespace" | "namespaces" | "ns"
    )
}

pub async fn run(
    config: &Config,
    resource: Option<String>,
    name: Option<String>,
    file: Option<String>,
    directory: Option<String>,
    recursive: bool,
) -> Result<()> {
    let client = Client::new(config)?;

    // Check if using file/directory mode or resource/name mode
    if file.is_some() || directory.is_some() {
        // Batch delete from files
        run_batch_delete(&client, config, file, directory, recursive).await
    } else if let (Some(resource), Some(name)) = (resource, name) {
        // Single resource delete
        run_single_delete(&client, &resource, &name).await
    } else {
        anyhow::bail!(
            "Either specify <resource> <name> or use --file/-f or --directory/-d"
        )
    }
}

async fn run_single_delete(client: &Client, resource: &str, name: &str) -> Result<()> {
    // Special handling for namespaces
    if is_namespace(resource) {
        client.delete_namespace(name).await?;
        println!("namespace \"{}\" deleted", name);
    } else {
        let resource_type = normalize_resource_type(resource);
        client.delete(resource_type, name, None).await?;
        println!("{} \"{}\" deleted", resource, name);
    }

    Ok(())
}

async fn run_batch_delete(
    client: &Client,
    config: &Config,
    file: Option<String>,
    directory: Option<String>,
    recursive: bool,
) -> Result<()> {
    let source = InputSource::from_args(file, directory, recursive)?;
    let files = source.get_files()?;

    if files.is_empty() {
        println!("{}", "No YAML files found".yellow());
        return Ok(());
    }

    let default_ns = config.namespace();
    let mut deleted_count = 0;
    let mut error_count = 0;
    let mut errors: Vec<String> = Vec::new();

    for file_path in &files {
        let file_display = file_path.display().to_string();

        // Read and parse file
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                error_count += 1;
                errors.push(format!("{}: Failed to read: {}", file_display, e));
                continue;
            }
        };

        let resources = match parse_yaml_documents(&content) {
            Ok(r) => r,
            Err(e) => {
                error_count += 1;
                errors.push(format!("{}: Failed to parse YAML: {}", file_display, e));
                continue;
            }
        };

        // Delete each resource
        for resource in resources {
            let kind = resource
                .get("kind")
                .and_then(|k| k.as_str())
                .unwrap_or("Unknown");
            let name = resource
                .get("metadata")
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("<unnamed>");
            let namespace = resource
                .get("metadata")
                .and_then(|m| m.get("namespace"))
                .and_then(|n| n.as_str())
                .unwrap_or(&default_ns);

            if name == "<unnamed>" {
                error_count += 1;
                errors.push(format!(
                    "{}: {} missing metadata.name",
                    file_display, kind
                ));
                continue;
            }

            // Determine resource type
            let resource_type = match kind.to_lowercase().as_str() {
                "goal" => "goals",
                "plan" => "plans",
                "capability" => "capabilities",
                "binding" => "bindings",
                "execution" => "executions",
                "namespace" => {
                    // Delete namespace directly
                    match client.delete_namespace(name).await {
                        Ok(_) => {
                            deleted_count += 1;
                            println!(
                                "namespace \"{}\" {}",
                                name,
                                "deleted".green()
                            );
                        }
                        Err(e) => {
                            // Ignore "not found" errors for idempotent delete
                            let err_str = e.to_string();
                            if err_str.contains("404") || err_str.contains("not found") {
                                println!(
                                    "namespace \"{}\" {} (already deleted)",
                                    name,
                                    "skipped".yellow()
                                );
                            } else {
                                error_count += 1;
                                errors.push(format!(
                                    "namespace/{}: {}",
                                    name, e
                                ));
                            }
                        }
                    }
                    continue;
                }
                _ => {
                    error_count += 1;
                    errors.push(format!(
                        "{}: Unknown resource kind '{}'",
                        file_display, kind
                    ));
                    continue;
                }
            };

            // Delete the resource
            match client.delete(resource_type, name, Some(namespace)).await {
                Ok(_) => {
                    deleted_count += 1;
                    println!(
                        "{}/{}/{} {}",
                        namespace,
                        kind.to_lowercase(),
                        name,
                        "deleted".green()
                    );
                }
                Err(e) => {
                    // Ignore "not found" errors for idempotent delete
                    let err_str = e.to_string();
                    if err_str.contains("404") || err_str.contains("not found") {
                        println!(
                            "{}/{}/{} {} (already deleted)",
                            namespace,
                            kind.to_lowercase(),
                            name,
                            "skipped".yellow()
                        );
                    } else {
                        error_count += 1;
                        errors.push(format!(
                            "{}/{}/{}: {}",
                            namespace,
                            kind.to_lowercase(),
                            name,
                            e
                        ));
                    }
                }
            }
        }
    }

    // Print errors
    for error in &errors {
        eprintln!("{} {}", "✗".red(), error);
    }

    // Summary
    if files.len() > 1 || deleted_count > 1 || error_count > 0 {
        println!();
        print!("{} resource(s) deleted", deleted_count);
        if error_count > 0 {
            print!(", {} error(s)", error_count);
        }
        println!();
    }

    if error_count > 0 {
        anyhow::bail!("{} error(s) occurred during delete", error_count);
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
