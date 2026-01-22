use anyhow::{Context, Result};
use colored::Colorize;
use planspec_core::Validator;
use serde_json::Value;
use similar::{ChangeTag, TextDiff};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::Command;
use tempfile::NamedTempFile;

use crate::client::{Client, Config};

/// Normalize singular resource type to plural for API
fn normalize_resource_type(resource: &str) -> Result<&'static str> {
    match resource.to_lowercase().as_str() {
        "goal" | "goals" => Ok("goals"),
        "plan" | "plans" => Ok("plans"),
        "capability" | "capabilities" => Ok("capabilities"),
        "binding" | "bindings" => Ok("bindings"),
        "execution" | "executions" => Ok("executions"),
        _ => anyhow::bail!("Unknown resource type: {}", resource),
    }
}

/// Edit a resource interactively
pub async fn run(
    config: &Config,
    resource: &str,
    name: &str,
    editor: Option<String>,
    _output_format: &str,
) -> Result<()> {
    let resource_type = normalize_resource_type(resource)?;
    let client = Client::new(config)?;
    let namespace = config.namespace();

    // 1. Fetch the resource from the server
    println!("Fetching {}/{}...", resource_type, name);
    let original = client.get(resource_type, name, Some(&namespace)).await?;

    // 2. Strip server-managed fields for editing
    let editable = strip_server_fields(&original);

    // 3. Convert to YAML for editing
    let original_yaml = serde_yaml::to_string(&editable)
        .context("Failed to serialize resource to YAML")?;

    // 4. Create temp file and write content
    let mut temp_file = NamedTempFile::with_suffix(".yaml")
        .context("Failed to create temp file")?;
    temp_file
        .write_all(original_yaml.as_bytes())
        .context("Failed to write to temp file")?;
    temp_file.flush()?;

    // 5. Determine editor
    let editor_cmd = editor
        .or_else(|| env::var("EDITOR").ok())
        .or_else(|| env::var("VISUAL").ok())
        .unwrap_or_else(|| "vi".to_string());

    // 6. Edit loop
    loop {
        // Open editor
        let status = Command::new(&editor_cmd)
            .arg(temp_file.path())
            .status()
            .with_context(|| format!("Failed to open editor: {}", editor_cmd))?;

        if !status.success() {
            anyhow::bail!("Editor exited with error");
        }

        // Read edited content
        let edited_yaml = fs::read_to_string(temp_file.path())
            .context("Failed to read edited file")?;

        // Check if content changed
        if edited_yaml.trim() == original_yaml.trim() {
            println!("{}", "Edit cancelled, no changes made.".yellow());
            return Ok(());
        }

        // Parse the edited YAML
        let edited: Value = match serde_yaml::from_str(&edited_yaml) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{} Invalid YAML: {}", "✗".red(), e);
                if !prompt_retry()? {
                    println!("{}", "Edit cancelled.".yellow());
                    return Ok(());
                }
                continue;
            }
        };

        // 7. Validate locally
        let validator = Validator::new()
            .map_err(|e| anyhow::anyhow!("Failed to create validator: {}", e))?;

        if let Err(errors) = validator.validate_json(&edited) {
            eprintln!("{} Validation failed:", "✗".red());
            for error in errors {
                eprintln!("  {} {}", "→".yellow(), error);
            }
            if !prompt_retry()? {
                println!("{}", "Edit cancelled.".yellow());
                return Ok(());
            }
            continue;
        }

        // 8. Show diff
        println!("\n{}", "Changes:".bold());
        show_diff(&original_yaml, &edited_yaml);

        // 9. Confirm and apply
        if !prompt_apply()? {
            if prompt_retry()? {
                continue;
            }
            println!("{}", "Edit cancelled.".yellow());
            return Ok(());
        }

        // 10. Apply the changes
        println!("\nApplying changes...");

        // Restore server-managed fields
        let mut to_apply = edited.clone();
        restore_server_fields(&mut to_apply, &original);

        // Use apply endpoint
        match client.apply(&namespace, &[to_apply]).await {
            Ok(result) => {
                if let Some(applied) = result.get("applied").and_then(|a| a.as_array()) {
                    for item in applied {
                        let action = item.get("action").and_then(|a| a.as_str()).unwrap_or("");
                        let action_colored = match action {
                            "created" => "created".green(),
                            "updated" => "configured".yellow(),
                            "unchanged" => "unchanged".blue(),
                            _ => action.normal(),
                        };
                        println!(
                            "{}/{} {}",
                            resource_type,
                            name,
                            action_colored
                        );
                    }
                }
                if let Some(errors) = result.get("errors").and_then(|e| e.as_array()) {
                    if !errors.is_empty() {
                        for error in errors {
                            let msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
                            eprintln!("{} {}", "✗".red(), msg);
                        }
                        if prompt_retry()? {
                            continue;
                        }
                        return Ok(());
                    }
                }
                return Ok(());
            }
            Err(e) => {
                eprintln!("{} Apply failed: {}", "✗".red(), e);

                // Check for conflict
                if e.to_string().contains("409") || e.to_string().contains("conflict") {
                    eprintln!(
                        "\n{} Resource was modified by another process.",
                        "⚠".yellow()
                    );
                    eprintln!("Please re-run the edit command to get the latest version.");
                    return Ok(());
                }

                if !prompt_retry()? {
                    println!("{}", "Edit cancelled.".yellow());
                    return Ok(());
                }
                continue;
            }
        }
    }
}

/// Strip server-managed fields from a resource for editing
fn strip_server_fields(resource: &Value) -> Value {
    let mut result = resource.clone();

    if let Some(obj) = result.as_object_mut() {
        // Remove status
        obj.remove("status");

        // Clean up metadata
        if let Some(metadata) = obj.get_mut("metadata").and_then(|m| m.as_object_mut()) {
            metadata.remove("creationTimestamp");
            metadata.remove("uid");
            metadata.remove("resourceVersion");
            metadata.remove("generation");
            metadata.remove("managedFields");
            metadata.remove("selfLink");
        }
    }

    result
}

/// Restore server-managed fields needed for update
fn restore_server_fields(edited: &mut Value, original: &Value) {
    if let (Some(edited_obj), Some(original_obj)) = (edited.as_object_mut(), original.as_object()) {
        // Restore metadata fields needed for concurrency control
        if let (Some(edited_meta), Some(original_meta)) = (
            edited_obj.get_mut("metadata").and_then(|m| m.as_object_mut()),
            original_obj.get("metadata").and_then(|m| m.as_object()),
        ) {
            // Copy resourceVersion for optimistic concurrency
            if let Some(rv) = original_meta.get("resourceVersion") {
                edited_meta.insert("resourceVersion".to_string(), rv.clone());
            }
            // Copy uid
            if let Some(uid) = original_meta.get("uid") {
                edited_meta.insert("uid".to_string(), uid.clone());
            }
        }
    }
}

/// Show a colorized diff between two strings
fn show_diff(original: &str, edited: &str) {
    let diff = TextDiff::from_lines(original, edited);

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => {
                print!("{}", format!("-{}", change).red());
            }
            ChangeTag::Insert => {
                print!("{}", format!("+{}", change).green());
            }
            ChangeTag::Equal => {
                print!(" {}", change);
            }
        }
    }
}

/// Prompt user to retry editing
fn prompt_retry() -> Result<bool> {
    print!("\nRe-edit? [y/n]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_lowercase().starts_with('y'))
}

/// Prompt user to apply changes
fn prompt_apply() -> Result<bool> {
    print!("\nApply these changes? [y/n]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_lowercase().starts_with('y'))
}
