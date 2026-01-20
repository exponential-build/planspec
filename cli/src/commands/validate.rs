use anyhow::{Context, Result};
use colored::Colorize;
use planspec_core::Validator;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Validate resources from a file against JSON schemas (offline)
pub fn run(file: &str, output_format: &str) -> Result<()> {
    let path = Path::new(file);
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", file))?;

    // Parse YAML (supports multi-document)
    let resources = parse_yaml_documents(&content)?;

    if resources.is_empty() {
        println!("{}", "No resources found in file".yellow());
        return Ok(());
    }

    let validator =
        Validator::new().map_err(|e| anyhow::anyhow!("Failed to create validator: {}", e))?;
    let mut all_valid = true;
    let mut results = Vec::new();

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
                results.push(ValidationResult {
                    index: i + 1,
                    kind: kind.to_string(),
                    name: name.to_string(),
                    valid: true,
                    errors: vec![],
                });
            }
            Err(errors) => {
                all_valid = false;
                results.push(ValidationResult {
                    index: i + 1,
                    kind: kind.to_string(),
                    name: name.to_string(),
                    valid: false,
                    errors: errors.iter().map(|e| e.to_string()).collect(),
                });
            }
        }
    }

    // Output results
    match output_format {
        "json" => output_json(&results)?,
        "yaml" => output_yaml(&results)?,
        _ => output_table(&results),
    }

    if all_valid {
        println!(
            "\n{} {} resource(s) validated successfully",
            "✓".green(),
            resources.len()
        );
        Ok(())
    } else {
        let invalid_count = results.iter().filter(|r| !r.valid).count();
        println!(
            "\n{} {} of {} resource(s) have validation errors",
            "✗".red(),
            invalid_count,
            resources.len()
        );
        anyhow::bail!("Validation failed")
    }
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

fn output_json(results: &[ValidationResult]) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(results)?);
    Ok(())
}

fn output_yaml(results: &[ValidationResult]) -> Result<()> {
    println!("{}", serde_yaml::to_string(results)?);
    Ok(())
}

fn output_table(results: &[ValidationResult]) {
    for result in results {
        let status = if result.valid {
            "✓".green().to_string()
        } else {
            "✗".red().to_string()
        };

        println!(
            "{} {}/{} {}",
            status,
            result.kind,
            result.name,
            if result.valid {
                "valid".green().to_string()
            } else {
                "invalid".red().to_string()
            }
        );

        for error in &result.errors {
            println!("  {} {}", "→".yellow(), error);
        }
    }
}
