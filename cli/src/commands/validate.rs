use anyhow::{Context, Result};
use colored::Colorize;
use planspec_core::Validator;
use serde_json::Value;
use std::fs;

use crate::commands::utils::InputSource;

/// Validate resources from a file or directory against JSON schemas (offline)
pub fn run(
    file: Option<String>,
    directory: Option<String>,
    recursive: bool,
    output_format: &str,
) -> Result<()> {
    let source = InputSource::from_args(file, directory, recursive)?;
    let files = source.get_files()?;

    if files.is_empty() {
        println!("{}", "No YAML files found".yellow());
        return Ok(());
    }

    let validator =
        Validator::new().map_err(|e| anyhow::anyhow!("Failed to create validator: {}", e))?;

    let mut all_results: Vec<FileValidationResult> = Vec::new();
    let mut all_valid = true;

    for file_path in &files {
        let file_display = file_path.display().to_string();

        // Read file
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                all_valid = false;
                all_results.push(FileValidationResult {
                    file: file_display,
                    resources: vec![],
                    file_error: Some(format!("Failed to read: {}", e)),
                });
                continue;
            }
        };

        // Parse YAML
        let resources = match parse_yaml_documents(&content) {
            Ok(r) => r,
            Err(e) => {
                all_valid = false;
                all_results.push(FileValidationResult {
                    file: file_display,
                    resources: vec![],
                    file_error: Some(format!("Failed to parse YAML: {}", e)),
                });
                continue;
            }
        };

        if resources.is_empty() {
            all_results.push(FileValidationResult {
                file: file_display,
                resources: vec![],
                file_error: None,
            });
            continue;
        }

        // Validate each resource
        let mut resource_results = Vec::new();
        for (i, resource) in resources.iter().enumerate() {
            let kind = resource
                .get("kind")
                .and_then(|k| k.as_str())
                .unwrap_or("Unknown");
            let name = resource
                .get("metadata")
                .and_then(|m| m.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("<unnamed>");

            match validator.validate_json(resource) {
                Ok(()) => {
                    resource_results.push(ValidationResult {
                        index: i + 1,
                        kind: kind.to_string(),
                        name: name.to_string(),
                        valid: true,
                        errors: vec![],
                    });
                }
                Err(errors) => {
                    all_valid = false;
                    resource_results.push(ValidationResult {
                        index: i + 1,
                        kind: kind.to_string(),
                        name: name.to_string(),
                        valid: false,
                        errors: errors.iter().map(|e| e.to_string()).collect(),
                    });
                }
            }
        }

        all_results.push(FileValidationResult {
            file: file_display,
            resources: resource_results,
            file_error: None,
        });
    }

    // Output results
    match output_format {
        "json" => output_json(&all_results)?,
        "yaml" => output_yaml(&all_results)?,
        _ => output_table(&all_results, files.len() > 1),
    }

    // Summary
    let total_resources: usize = all_results.iter().map(|f| f.resources.len()).sum();
    let invalid_count: usize = all_results
        .iter()
        .map(|f| f.resources.iter().filter(|r| !r.valid).count())
        .sum();
    let file_errors: usize = all_results.iter().filter(|f| f.file_error.is_some()).count();

    if all_valid {
        println!(
            "\n{} {} resource(s) validated successfully{}",
            "✓".green(),
            total_resources,
            if files.len() > 1 {
                format!(" from {} file(s)", files.len())
            } else {
                String::new()
            }
        );
        Ok(())
    } else {
        println!(
            "\n{} {} of {} resource(s) have validation errors{}",
            "✗".red(),
            invalid_count + file_errors,
            total_resources + file_errors,
            if files.len() > 1 {
                format!(" across {} file(s)", files.len())
            } else {
                String::new()
            }
        );
        anyhow::bail!("Validation failed")
    }
}

#[derive(serde::Serialize)]
struct FileValidationResult {
    file: String,
    resources: Vec<ValidationResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_error: Option<String>,
}

#[derive(serde::Serialize)]
struct ValidationResult {
    index: usize,
    kind: String,
    name: String,
    valid: bool,
    errors: Vec<String>,
}

fn parse_yaml_documents(content: &str) -> Result<Vec<Value>> {
    let mut resources = Vec::new();

    for doc in serde_yaml::Deserializer::from_str(content) {
        let value: Value =
            serde::Deserialize::deserialize(doc).context("Failed to parse YAML document")?;

        // Skip null documents (empty docs between ---)
        if value.is_null() {
            continue;
        }

        // Handle List kind
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

fn output_json(results: &[FileValidationResult]) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(results)?);
    Ok(())
}

fn output_yaml(results: &[FileValidationResult]) -> Result<()> {
    println!("{}", serde_yaml::to_string(results)?);
    Ok(())
}

fn output_table(results: &[FileValidationResult], show_filenames: bool) {
    for file_result in results {
        // Show file-level errors
        if let Some(ref error) = file_result.file_error {
            if show_filenames {
                eprintln!("{} {}", "✗".red(), file_result.file.dimmed());
            }
            eprintln!("  {} {}", "→".yellow(), error);
            continue;
        }

        // Show resource results
        for result in &file_result.resources {
            let status = if result.valid {
                "✓".green().to_string()
            } else {
                "✗".red().to_string()
            };

            let validity = if result.valid {
                "valid".green().to_string()
            } else {
                "invalid".red().to_string()
            };

            if show_filenames {
                println!(
                    "{} {}/{} {} {}",
                    status,
                    result.kind,
                    result.name,
                    validity,
                    format!("({})", file_result.file).dimmed()
                );
            } else {
                println!("{} {}/{} {}", status, result.kind, result.name, validity);
            }

            for error in &result.errors {
                println!("  {} {}", "→".yellow(), error);
            }
        }
    }
}
