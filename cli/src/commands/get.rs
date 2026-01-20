use anyhow::Result;
use colored::Colorize;
use serde_json::Value;
use tabled::{Table, Tabled};

use crate::client::{Client, Config};

/// Get one or more resources
pub async fn run(
    config: &Config,
    resource: &str,
    name: Option<String>,
    selector: Option<String>,
    all_namespaces: bool,
    series: Option<String>,
    output_format: &str,
) -> Result<()> {
    let client = Client::new(config)?;
    let resource_type = normalize_resource_type(resource);

    // Special handling for namespaces
    if resource_type == "namespaces" {
        let result = client.list_namespaces().await?;
        return match output_format {
            "json" => {
                println!("{}", serde_json::to_string_pretty(&result)?);
                Ok(())
            }
            "yaml" => {
                println!("{}", serde_yaml::to_string(&result)?);
                Ok(())
            }
            _ => {
                print_namespaces(&result);
                Ok(())
            }
        };
    }

    let mut result = if let Some(name) = name {
        client.get(&resource_type, &name, None).await?
    } else {
        client
            .list(&resource_type, None, selector.as_deref(), all_namespaces)
            .await?
    };

    // Filter plans by series if specified (client-side filtering)
    if let Some(ref series_filter) = series {
        if resource_type == "plans" {
            if let Some(items) = result.get_mut("items").and_then(|i| i.as_array_mut()) {
                items.retain(|item| {
                    item.get("spec")
                        .and_then(|s| s.get("series"))
                        .and_then(|s| s.as_str())
                        .map(|s| s == series_filter)
                        .unwrap_or(false)
                });
            }
        }
    }

    // Sort plans by (series asc, version desc)
    if resource_type == "plans" {
        if let Some(items) = result.get_mut("items").and_then(|i| i.as_array_mut()) {
            items.sort_by(|a, b| {
                let a_spec = a.get("spec").unwrap_or(&Value::Null);
                let b_spec = b.get("spec").unwrap_or(&Value::Null);

                let a_series = a_spec.get("series").and_then(|s| s.as_str()).unwrap_or("");
                let b_series = b_spec.get("series").and_then(|s| s.as_str()).unwrap_or("");

                // Primary sort: series ascending
                match a_series.cmp(b_series) {
                    std::cmp::Ordering::Equal => {
                        // Secondary sort: version descending (numeric if possible)
                        let a_version = a_spec
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("0");
                        let b_version = b_spec
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("0");

                        // Try numeric comparison first
                        let a_num: Result<i64, _> = a_version.parse();
                        let b_num: Result<i64, _> = b_version.parse();

                        match (a_num, b_num) {
                            (Ok(a), Ok(b)) => b.cmp(&a),   // Descending
                            _ => b_version.cmp(a_version), // Fall back to string comparison, descending
                        }
                    }
                    other => other,
                }
            });
        }
    }

    match output_format {
        "json" => println!("{}", serde_json::to_string_pretty(&result)?),
        "yaml" => println!("{}", serde_yaml::to_string(&result)?),
        _ => output_table(&resource_type, &result, all_namespaces),
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
        "namespace" | "namespaces" | "ns" => "namespaces",
        other => other,
    }
    .to_string()
}

#[derive(Tabled)]
struct NamespaceRow {
    #[tabled(rename = "NAME")]
    name: String,
}

fn print_namespaces(result: &Value) {
    if let Some(items) = result.get("items").and_then(|i| i.as_array()) {
        if items.is_empty() {
            println!("No namespaces found");
            return;
        }

        let rows: Vec<NamespaceRow> = items
            .iter()
            .map(|item| {
                let metadata = item.get("metadata").unwrap_or(&Value::Null);
                NamespaceRow {
                    name: metadata
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string(),
                }
            })
            .collect();

        let table = Table::new(rows);
        println!("{}", table);
    } else {
        println!("No namespaces found");
    }
}

fn output_table(resource_type: &str, result: &Value, show_namespace: bool) {
    // Check if this is a list or single item
    if let Some(items) = result.get("items").and_then(|i| i.as_array()) {
        if items.is_empty() {
            println!("No resources found");
            return;
        }

        match resource_type {
            "goals" => print_goals(items, show_namespace),
            "plans" => print_plans(items, show_namespace),
            "capabilities" => print_capabilities(items, show_namespace),
            "bindings" => print_bindings(items, show_namespace),
            "executions" => print_executions(items, show_namespace),
            _ => print_generic(items, show_namespace),
        }
    } else {
        // Single item
        match resource_type {
            "goals" => print_goals(std::slice::from_ref(result), show_namespace),
            "plans" => print_plans(std::slice::from_ref(result), show_namespace),
            "capabilities" => print_capabilities(std::slice::from_ref(result), show_namespace),
            "bindings" => print_bindings(std::slice::from_ref(result), show_namespace),
            "executions" => print_executions(std::slice::from_ref(result), show_namespace),
            _ => print_generic(std::slice::from_ref(result), show_namespace),
        }
    }
}

#[derive(Tabled)]
struct GoalRow {
    #[tabled(rename = "NAMESPACE")]
    namespace: String,
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "PHASE")]
    phase: String,
    #[tabled(rename = "ACTIVE PLAN")]
    active_plan: String,
    #[tabled(rename = "AGE")]
    age: String,
}

fn print_goals(items: &[Value], show_namespace: bool) {
    let rows: Vec<GoalRow> = items
        .iter()
        .map(|item| {
            let metadata = item.get("metadata").unwrap_or(&Value::Null);
            let status = item.get("status").unwrap_or(&Value::Null);

            GoalRow {
                namespace: if show_namespace {
                    metadata
                        .get("namespace")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                },
                name: metadata
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                phase: colorize_phase(status.get("phase").and_then(|p| p.as_str()).unwrap_or("")),
                active_plan: status
                    .get("activePlanRef")
                    .and_then(|r| r.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("-")
                    .to_string(),
                age: calculate_age(metadata.get("creationTimestamp")),
            }
        })
        .collect();

    let mut table = Table::new(rows);
    if !show_namespace {
        // Hide namespace column if not needed
        table.with(tabled::settings::Disable::column(
            tabled::settings::object::Columns::first(),
        ));
    }
    println!("{}", table);
}

#[derive(Tabled)]
struct PlanRow {
    #[tabled(rename = "NAMESPACE")]
    namespace: String,
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "PHASE")]
    phase: String,
    #[tabled(rename = "NODES")]
    nodes: String,
    #[tabled(rename = "SERIES")]
    series: String,
    #[tabled(rename = "VERSION")]
    version: String,
}

fn print_plans(items: &[Value], show_namespace: bool) {
    let rows: Vec<PlanRow> = items
        .iter()
        .map(|item| {
            let metadata = item.get("metadata").unwrap_or(&Value::Null);
            let spec = item.get("spec").unwrap_or(&Value::Null);
            let status = item.get("status").unwrap_or(&Value::Null);

            PlanRow {
                namespace: if show_namespace {
                    metadata
                        .get("namespace")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                },
                name: metadata
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                phase: colorize_phase(status.get("phase").and_then(|p| p.as_str()).unwrap_or("")),
                nodes: status
                    .get("nodeCount")
                    .and_then(|n| n.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or("-".to_string()),
                series: spec
                    .get("series")
                    .and_then(|s| s.as_str())
                    .unwrap_or("-")
                    .to_string(),
                version: spec
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string(),
            }
        })
        .collect();

    let mut table = Table::new(rows);
    if !show_namespace {
        table.with(tabled::settings::Disable::column(
            tabled::settings::object::Columns::first(),
        ));
    }
    println!("{}", table);
}

#[derive(Tabled)]
struct CapabilityRow {
    #[tabled(rename = "NAMESPACE")]
    namespace: String,
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "PHASE")]
    phase: String,
    #[tabled(rename = "CATEGORY")]
    category: String,
}

fn print_capabilities(items: &[Value], show_namespace: bool) {
    let rows: Vec<CapabilityRow> = items
        .iter()
        .map(|item| {
            let metadata = item.get("metadata").unwrap_or(&Value::Null);
            let spec = item.get("spec").unwrap_or(&Value::Null);
            let status = item.get("status").unwrap_or(&Value::Null);

            CapabilityRow {
                namespace: if show_namespace {
                    metadata
                        .get("namespace")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                },
                name: metadata
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                phase: colorize_phase(status.get("phase").and_then(|p| p.as_str()).unwrap_or("")),
                category: spec
                    .get("category")
                    .and_then(|c| c.as_str())
                    .unwrap_or("-")
                    .to_string(),
            }
        })
        .collect();

    let mut table = Table::new(rows);
    if !show_namespace {
        table.with(tabled::settings::Disable::column(
            tabled::settings::object::Columns::first(),
        ));
    }
    println!("{}", table);
}

#[derive(Tabled)]
struct BindingRow {
    #[tabled(rename = "NAMESPACE")]
    namespace: String,
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "PHASE")]
    phase: String,
    #[tabled(rename = "RULES")]
    rules: String,
}

fn print_bindings(items: &[Value], show_namespace: bool) {
    let rows: Vec<BindingRow> = items
        .iter()
        .map(|item| {
            let metadata = item.get("metadata").unwrap_or(&Value::Null);
            let spec = item.get("spec").unwrap_or(&Value::Null);
            let status = item.get("status").unwrap_or(&Value::Null);

            BindingRow {
                namespace: if show_namespace {
                    metadata
                        .get("namespace")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                },
                name: metadata
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                phase: colorize_phase(status.get("phase").and_then(|p| p.as_str()).unwrap_or("")),
                rules: spec
                    .get("rules")
                    .and_then(|r| r.as_array())
                    .map(|r| r.len().to_string())
                    .unwrap_or("-".to_string()),
            }
        })
        .collect();

    let mut table = Table::new(rows);
    if !show_namespace {
        table.with(tabled::settings::Disable::column(
            tabled::settings::object::Columns::first(),
        ));
    }
    println!("{}", table);
}

#[derive(Tabled)]
struct ExecutionRow {
    #[tabled(rename = "NAMESPACE")]
    namespace: String,
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "PHASE")]
    phase: String,
    #[tabled(rename = "PLAN")]
    plan: String,
    #[tabled(rename = "AGE")]
    age: String,
}

fn print_executions(items: &[Value], show_namespace: bool) {
    let rows: Vec<ExecutionRow> = items
        .iter()
        .map(|item| {
            let metadata = item.get("metadata").unwrap_or(&Value::Null);
            let spec = item.get("spec").unwrap_or(&Value::Null);
            let status = item.get("status").unwrap_or(&Value::Null);

            ExecutionRow {
                namespace: if show_namespace {
                    metadata
                        .get("namespace")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                },
                name: metadata
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                phase: colorize_phase(status.get("phase").and_then(|p| p.as_str()).unwrap_or("")),
                plan: spec
                    .get("planRef")
                    .and_then(|r| r.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("-")
                    .to_string(),
                age: calculate_age(metadata.get("creationTimestamp")),
            }
        })
        .collect();

    let mut table = Table::new(rows);
    if !show_namespace {
        table.with(tabled::settings::Disable::column(
            tabled::settings::object::Columns::first(),
        ));
    }
    println!("{}", table);
}

fn print_generic(items: &[Value], _show_namespace: bool) {
    for item in items {
        let metadata = item.get("metadata").unwrap_or(&Value::Null);
        let kind = item
            .get("kind")
            .and_then(|k| k.as_str())
            .unwrap_or("Unknown");
        let name = metadata.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let ns = metadata
            .get("namespace")
            .and_then(|n| n.as_str())
            .unwrap_or("");
        println!("{}/{}  ({})", kind, name, ns);
    }
}

fn colorize_phase(phase: &str) -> String {
    match phase {
        "Succeeded" | "Ready" | "Available" => phase.green().to_string(),
        "Running" | "Executing" | "Planning" => phase.blue().to_string(),
        "Pending" => phase.yellow().to_string(),
        "Failed" | "Invalid" | "Unresolved" => phase.red().to_string(),
        "Cancelled" => phase.dimmed().to_string(),
        _ => phase.to_string(),
    }
}

fn calculate_age(timestamp: Option<&Value>) -> String {
    let Some(ts) = timestamp.and_then(|t| t.as_str()) else {
        return "-".to_string();
    };

    let Ok(created) = chrono::DateTime::parse_from_rfc3339(ts) else {
        return "-".to_string();
    };

    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(created);

    if duration.num_days() > 0 {
        format!("{}d", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{}h", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{}m", duration.num_minutes())
    } else {
        format!("{}s", duration.num_seconds())
    }
}
