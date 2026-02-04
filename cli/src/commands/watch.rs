use anyhow::Result;
use colored::Colorize;

use crate::client::Config;

/// Watch for changes to resources
pub async fn run(
    config: &Config,
    resource: &str,
    selector: Option<String>,
    _output_format: &str,
) -> Result<()> {
    let resource_type = normalize_resource_type(resource);
    let namespace = config.namespace();
    let server = config.server_url();

    let mut url = format!(
        "{}/apis/planspec.io/v1alpha1/namespaces/{}/{}/watch",
        server, namespace, resource_type
    );

    if let Some(sel) = &selector {
        url.push_str(&format!("?labelSelector={}", sel));
    }

    println!(
        "{} Watching {} in namespace {}...",
        "→".blue(),
        resource_type,
        namespace
    );
    println!("{}", "-".repeat(60));

    // Use reqwest to stream the response
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await?;
        anyhow::bail!("Watch failed: {} - {}", status, body);
    }

    // Read line by line from the response body
    let body = response.text().await?;

    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(event) = serde_json::from_str::<serde_json::Value>(line) {
            print_event(&event);
        }
    }

    // Note: In a real implementation, you'd use Server-Sent Events or
    // WebSocket streaming. This simplified version just reads the full response.
    println!("{}", "Watch ended".dimmed());

    Ok(())
}

fn normalize_resource_type(resource: &str) -> String {
    match resource.to_lowercase().as_str() {
        "goal" | "goals" => "goals",
        "plan" | "plans" => "plans",
        "capability" | "capabilities" => "capabilities",
        "binding" | "bindings" => "bindings",
        "execution" | "executions" => "executions",
        other => other,
    }
    .to_string()
}

fn print_event(event: &serde_json::Value) {
    let event_type = event
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("UNKNOWN");

    let object = event.get("object").unwrap_or(&serde_json::Value::Null);
    let kind = object.get("kind").and_then(|k| k.as_str()).unwrap_or("");
    let metadata = object.get("metadata").unwrap_or(&serde_json::Value::Null);
    let name = metadata.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let namespace = metadata
        .get("namespace")
        .and_then(|n| n.as_str())
        .unwrap_or("");

    let status = object.get("status").unwrap_or(&serde_json::Value::Null);
    let phase = status.get("phase").and_then(|p| p.as_str()).unwrap_or("");

    let event_colored = match event_type {
        "ADDED" => event_type.green(),
        "MODIFIED" => event_type.yellow(),
        "DELETED" => event_type.red(),
        _ => event_type.normal(),
    };

    let phase_colored = match phase {
        "Succeeded" | "Ready" | "Available" => phase.green(),
        "Running" | "Executing" | "Planning" => phase.blue(),
        "Pending" => phase.yellow(),
        "Failed" | "Invalid" => phase.red(),
        "Cancelled" => phase.dimmed(),
        _ => phase.normal(),
    };

    let now = chrono::Local::now().format("%H:%M:%S");
    println!(
        "[{}] {} {}/{} ({}) {}",
        now, event_colored, kind, name, namespace, phase_colored
    );
}
