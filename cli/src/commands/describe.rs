use anyhow::Result;
use colored::Colorize;
use serde_json::Value;

use crate::client::{Client, Config};

/// Show detailed information about a resource
pub async fn run(config: &Config, resource: &str, name: &str, output_format: &str) -> Result<()> {
    let client = Client::new(config)?;
    let resource_type = normalize_resource_type(resource);

    let result = client.get(&resource_type, name, None).await?;

    match output_format {
        "json" => println!("{}", serde_json::to_string_pretty(&result)?),
        "yaml" => println!("{}", serde_yaml::to_string(&result)?),
        _ => describe_resource(&resource_type, &result),
    }

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

fn describe_resource(resource_type: &str, resource: &Value) {
    let metadata = resource.get("metadata").unwrap_or(&Value::Null);
    let spec = resource.get("spec").unwrap_or(&Value::Null);
    let status = resource.get("status").unwrap_or(&Value::Null);

    // Common header
    let kind = resource.get("kind").and_then(|k| k.as_str()).unwrap_or("Unknown");
    let name = metadata.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let namespace = metadata.get("namespace").and_then(|n| n.as_str()).unwrap_or("");

    println!("{}: {}", "Name".bold(), name);
    println!("{}: {}", "Namespace".bold(), namespace);
    println!("{}: {}", "Kind".bold(), kind);

    // Labels
    if let Some(labels) = metadata.get("labels").and_then(|l| l.as_object()) {
        println!("{}:", "Labels".bold());
        for (key, value) in labels {
            println!("  {}: {}", key, value.as_str().unwrap_or(""));
        }
    }

    // Annotations
    if let Some(annotations) = metadata.get("annotations").and_then(|a| a.as_object()) {
        println!("{}:", "Annotations".bold());
        for (key, value) in annotations {
            println!("  {}: {}", key, value.as_str().unwrap_or(""));
        }
    }

    // System fields
    if let Some(uid) = metadata.get("uid").and_then(|u| u.as_str()) {
        println!("{}: {}", "UID".bold(), uid);
    }
    if let Some(rv) = metadata.get("resourceVersion").and_then(|r| r.as_str()) {
        println!("{}: {}", "Resource Version".bold(), rv);
    }
    if let Some(gen) = metadata.get("generation").and_then(|g| g.as_i64()) {
        println!("{}: {}", "Generation".bold(), gen);
    }
    if let Some(ts) = metadata.get("creationTimestamp").and_then(|t| t.as_str()) {
        println!("{}: {}", "Created".bold(), ts);
    }

    println!();

    // Resource-specific sections
    match resource_type {
        "goals" => describe_goal(spec, status),
        "plans" => describe_plan(spec, status),
        "capabilities" => describe_capability(spec, status),
        "bindings" => describe_binding(spec, status),
        "executions" => describe_execution(spec, status),
        _ => {
            println!("{}:", "Spec".bold());
            println!("{}", serde_yaml::to_string(spec).unwrap_or_default());
        }
    }

    // Conditions
    if let Some(conditions) = status.get("conditions").and_then(|c| c.as_array()) {
        println!("{}:", "Conditions".bold());
        for cond in conditions {
            let ctype = cond.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let cstatus = cond.get("status").and_then(|s| s.as_str()).unwrap_or("");
            let reason = cond.get("reason").and_then(|r| r.as_str()).unwrap_or("");
            let message = cond.get("message").and_then(|m| m.as_str()).unwrap_or("");

            let status_colored = match cstatus {
                "True" => cstatus.green(),
                "False" => cstatus.red(),
                _ => cstatus.yellow(),
            };

            println!("  {} {} ({})", ctype.bold(), status_colored, reason);
            if !message.is_empty() {
                println!("    {}", message);
            }
        }
    }
}

fn describe_goal(spec: &Value, status: &Value) {
    println!("{}:", "Spec".bold());

    if let Some(desc) = spec.get("description").and_then(|d| d.as_str()) {
        println!("  {}: {}", "Description".bold(), desc);
    }

    if let Some(timeout) = spec.get("timeout").and_then(|t| t.as_str()) {
        println!("  {}: {}", "Timeout".bold(), timeout);
    }

    if let Some(priority) = spec.get("priority").and_then(|p| p.as_i64()) {
        println!("  {}: {}", "Priority".bold(), priority);
    }

    if let Some(constraints) = spec.get("constraints").and_then(|c| c.as_object()) {
        println!("  {}:", "Constraints".bold());
        for (key, value) in constraints {
            println!("    {}: {}", key, value);
        }
    }

    if let Some(criteria) = spec.get("acceptanceCriteria").and_then(|a| a.as_array()) {
        println!("  {}:", "Acceptance Criteria".bold());
        for (i, criterion) in criteria.iter().enumerate() {
            if let Some(desc) = criterion.get("description").and_then(|d| d.as_str()) {
                println!("    {}. {}", i + 1, desc);
            }
        }
    }

    println!();
    println!("{}:", "Status".bold());

    if let Some(phase) = status.get("phase").and_then(|p| p.as_str()) {
        println!("  {}: {}", "Phase".bold(), colorize_phase(phase));
    }

    if let Some(plan_ref) = status.get("activePlanRef") {
        if let Some(name) = plan_ref.get("name").and_then(|n| n.as_str()) {
            println!("  {}: {}", "Active Plan".bold(), name);
        }
    }

    if let Some(gen) = status.get("observedGeneration").and_then(|g| g.as_i64()) {
        println!("  {}: {}", "Observed Generation".bold(), gen);
    }
}

fn describe_plan(spec: &Value, status: &Value) {
    println!("{}:", "Spec".bold());

    if let Some(desc) = spec.get("description").and_then(|d| d.as_str()) {
        println!("  {}: {}", "Description".bold(), desc);
    }

    if let Some(goal_ref) = spec.get("goalRef") {
        if let Some(name) = goal_ref.get("name").and_then(|n| n.as_str()) {
            println!("  {}: {}", "Goal".bold(), name);
        }
    }

    if let Some(series) = spec.get("series").and_then(|s| s.as_str()) {
        println!("  {}: {}", "Series".bold(), series);
    }

    if let Some(version) = spec.get("version").and_then(|v| v.as_str()) {
        println!("  {}: {}", "Version".bold(), version);
    }

    // Graph summary
    if let Some(graph) = spec.get("graph") {
        if let Some(nodes) = graph.get("nodes").and_then(|n| n.as_array()) {
            println!();
            println!("  {} ({} nodes):", "Graph".bold(), nodes.len());
            for node in nodes {
                let id = node.get("id").and_then(|i| i.as_str()).unwrap_or("");
                let kind = node.get("kind").and_then(|k| k.as_str()).unwrap_or("");
                let desc = node.get("description").and_then(|d| d.as_str()).unwrap_or("");
                println!("    [{}] {} - {}", kind, id.cyan(), desc);
            }
        }

        if let Some(edges) = graph.get("edges").and_then(|e| e.as_array()) {
            println!();
            println!("  {} ({} edges):", "Dependencies".bold(), edges.len());
            for edge in edges {
                let from = edge.get("from").and_then(|f| f.as_str()).unwrap_or("");
                let to = edge.get("to").and_then(|t| t.as_str()).unwrap_or("");
                println!("    {} -> {}", from, to);
            }
        }
    }

    println!();
    println!("{}:", "Status".bold());

    if let Some(phase) = status.get("phase").and_then(|p| p.as_str()) {
        println!("  {}: {}", "Phase".bold(), colorize_phase(phase));
    }

    if let Some(count) = status.get("nodeCount").and_then(|n| n.as_i64()) {
        println!("  {}: {}", "Node Count".bold(), count);
    }
}

fn describe_capability(spec: &Value, status: &Value) {
    println!("{}:", "Spec".bold());

    if let Some(display) = spec.get("displayName").and_then(|d| d.as_str()) {
        println!("  {}: {}", "Display Name".bold(), display);
    }

    if let Some(desc) = spec.get("description").and_then(|d| d.as_str()) {
        println!("  {}: {}", "Description".bold(), desc);
    }

    if let Some(category) = spec.get("category").and_then(|c| c.as_str()) {
        println!("  {}: {}", "Category".bold(), category);
    }

    if let Some(inputs) = spec.get("inputs").and_then(|i| i.as_array()) {
        println!("  {}:", "Inputs".bold());
        for input in inputs {
            let name = input.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let ptype = input.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let required = input.get("required").and_then(|r| r.as_bool()).unwrap_or(false);
            let req_str = if required { " (required)" } else { "" };
            println!("    {} [{}]{}", name, ptype, req_str);
        }
    }

    if let Some(outputs) = spec.get("outputs").and_then(|o| o.as_array()) {
        println!("  {}:", "Outputs".bold());
        for output in outputs {
            let name = output.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let ptype = output.get("type").and_then(|t| t.as_str()).unwrap_or("");
            println!("    {} [{}]", name, ptype);
        }
    }

    println!();
    println!("{}:", "Status".bold());

    if let Some(phase) = status.get("phase").and_then(|p| p.as_str()) {
        println!("  {}: {}", "Phase".bold(), colorize_phase(phase));
    }
}

fn describe_binding(spec: &Value, status: &Value) {
    println!("{}:", "Spec".bold());

    if let Some(rules) = spec.get("rules").and_then(|r| r.as_array()) {
        println!("  {} ({}):", "Rules".bold(), rules.len());
        for (i, rule) in rules.iter().enumerate() {
            println!("  {}. Rule {}:", "", i + 1);

            if let Some(selector) = rule.get("selector") {
                if let Some(cap_ref) = selector.get("capabilityRef") {
                    if let Some(name) = cap_ref.get("name").and_then(|n| n.as_str()) {
                        println!("     Capability: {}", name);
                    }
                }
                if let Some(plan_ref) = selector.get("planRef") {
                    if let Some(name) = plan_ref.get("name").and_then(|n| n.as_str()) {
                        println!("     Plan: {}", name);
                    }
                }
                if let Some(node_id) = selector.get("nodeId").and_then(|n| n.as_str()) {
                    println!("     Node: {}", node_id);
                }
            }

            if let Some(target) = rule.get("target") {
                if let Some(provider) = target.get("provider").and_then(|p| p.as_str()) {
                    println!("     Provider: {}", provider.cyan());
                }
            }
        }
    }

    println!();
    println!("{}:", "Status".bold());

    if let Some(phase) = status.get("phase").and_then(|p| p.as_str()) {
        println!("  {}: {}", "Phase".bold(), colorize_phase(phase));
    }

    if let Some(count) = status.get("boundCount").and_then(|n| n.as_i64()) {
        println!("  {}: {}", "Bound Count".bold(), count);
    }

    if let Some(unbound) = status.get("unboundCapabilities").and_then(|u| u.as_array()) {
        if !unbound.is_empty() {
            println!("  {}:", "Unbound Capabilities".bold());
            for cap in unbound {
                if let Some(name) = cap.as_str() {
                    println!("    - {}", name.red());
                }
            }
        }
    }
}

fn describe_execution(spec: &Value, status: &Value) {
    println!("{}:", "Spec".bold());

    if let Some(goal_ref) = spec.get("goalRef") {
        if let Some(name) = goal_ref.get("name").and_then(|n| n.as_str()) {
            println!("  {}: {}", "Goal".bold(), name);
        }
    }

    if let Some(plan_ref) = spec.get("planRef") {
        if let Some(name) = plan_ref.get("name").and_then(|n| n.as_str()) {
            println!("  {}: {}", "Plan".bold(), name);
        }
    }

    if let Some(binding_ref) = spec.get("bindingRef") {
        if let Some(name) = binding_ref.get("name").and_then(|n| n.as_str()) {
            println!("  {}: {}", "Binding".bold(), name);
        }
    }

    if let Some(params) = spec.get("parameters").and_then(|p| p.as_object()) {
        if !params.is_empty() {
            println!("  {}:", "Parameters".bold());
            for (key, value) in params {
                println!("    {}: {}", key, value);
            }
        }
    }

    println!();
    println!("{}:", "Status".bold());

    if let Some(phase) = status.get("phase").and_then(|p| p.as_str()) {
        println!("  {}: {}", "Phase".bold(), colorize_phase(phase));
    }

    if let Some(run_id) = status.get("runId").and_then(|r| r.as_str()) {
        println!("  {}: {}", "Run ID".bold(), run_id);
    }

    if let Some(start) = status.get("startTime").and_then(|s| s.as_str()) {
        println!("  {}: {}", "Start Time".bold(), start);
    }

    if let Some(end) = status.get("completionTime").and_then(|e| e.as_str()) {
        println!("  {}: {}", "Completion Time".bold(), end);
    }

    // Node statuses
    if let Some(node_statuses) = status.get("nodeStatuses").and_then(|n| n.as_object()) {
        println!();
        println!("  {} ({}):", "Node Statuses".bold(), node_statuses.len());
        for (node_id, node_status) in node_statuses {
            let phase = node_status
                .get("phase")
                .and_then(|p| p.as_str())
                .unwrap_or("");
            let attempt = node_status
                .get("attempt")
                .and_then(|a| a.as_i64())
                .unwrap_or(0);

            println!(
                "    {} {} (attempt {})",
                node_id.cyan(),
                colorize_phase(phase),
                attempt
            );

            if let Some(msg) = node_status.get("message").and_then(|m| m.as_str()) {
                if !msg.is_empty() {
                    println!("      {}", msg);
                }
            }
        }
    }

    // Artifacts
    if let Some(artifacts) = status.get("artifacts").and_then(|a| a.as_array()) {
        if !artifacts.is_empty() {
            println!();
            println!("  {}:", "Artifacts".bold());
            for artifact in artifacts {
                let name = artifact.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let atype = artifact.get("type").and_then(|t| t.as_str()).unwrap_or("");
                println!("    {} [{}]", name, atype);
            }
        }
    }
}

fn colorize_phase(phase: &str) -> String {
    match phase {
        "Succeeded" | "Ready" | "Available" => phase.green().to_string(),
        "Running" | "Executing" | "Planning" => phase.blue().to_string(),
        "Pending" => phase.yellow().to_string(),
        "Failed" | "Invalid" | "Unresolved" => phase.red().to_string(),
        "Cancelled" | "Skipped" => phase.dimmed().to_string(),
        _ => phase.to_string(),
    }
}
