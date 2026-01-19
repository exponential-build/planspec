use serde_json::Value;
use tabled::{Table, Tabled};

/// Table output formatter for resources
#[allow(dead_code)]
pub struct TableFormatter;

impl TableFormatter {
    /// Format a list of resources as a table
    pub fn format_list(resources: &[Value], show_namespace: bool) -> String {
        if resources.is_empty() {
            return "No resources found".to_string();
        }

        // Detect resource type from first item
        let kind = resources
            .first()
            .and_then(|r| r.get("kind"))
            .and_then(|k| k.as_str())
            .unwrap_or("Unknown");

        match kind {
            "Goal" => format_goals(resources, show_namespace),
            "Plan" => format_plans(resources, show_namespace),
            "Capability" => format_capabilities(resources, show_namespace),
            "Binding" => format_bindings(resources, show_namespace),
            "Execution" => format_executions(resources, show_namespace),
            _ => format_generic(resources),
        }
    }
}

#[derive(Tabled)]
struct GenericRow {
    #[tabled(rename = "KIND")]
    kind: String,
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "NAMESPACE")]
    namespace: String,
}

fn format_generic(resources: &[Value]) -> String {
    let rows: Vec<GenericRow> = resources
        .iter()
        .map(|r| {
            let metadata = r.get("metadata").unwrap_or(&Value::Null);
            GenericRow {
                kind: r
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .unwrap_or("")
                    .to_string(),
                name: metadata
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                namespace: metadata
                    .get("namespace")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
            }
        })
        .collect();

    Table::new(rows).to_string()
}

#[derive(Tabled)]
struct GoalRow {
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "NAMESPACE")]
    namespace: String,
    #[tabled(rename = "PHASE")]
    phase: String,
    #[tabled(rename = "ACTIVE PLAN")]
    active_plan: String,
}

fn format_goals(resources: &[Value], show_namespace: bool) -> String {
    let rows: Vec<GoalRow> = resources
        .iter()
        .map(|r| {
            let metadata = r.get("metadata").unwrap_or(&Value::Null);
            let status = r.get("status").unwrap_or(&Value::Null);

            GoalRow {
                name: metadata
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                namespace: if show_namespace {
                    metadata
                        .get("namespace")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                },
                phase: status
                    .get("phase")
                    .and_then(|p| p.as_str())
                    .unwrap_or("-")
                    .to_string(),
                active_plan: status
                    .get("activePlanRef")
                    .and_then(|r| r.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("-")
                    .to_string(),
            }
        })
        .collect();

    Table::new(rows).to_string()
}

#[derive(Tabled)]
struct PlanRow {
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "NAMESPACE")]
    namespace: String,
    #[tabled(rename = "PHASE")]
    phase: String,
    #[tabled(rename = "NODES")]
    nodes: String,
    #[tabled(rename = "VERSION")]
    version: String,
}

fn format_plans(resources: &[Value], show_namespace: bool) -> String {
    let rows: Vec<PlanRow> = resources
        .iter()
        .map(|r| {
            let metadata = r.get("metadata").unwrap_or(&Value::Null);
            let spec = r.get("spec").unwrap_or(&Value::Null);
            let status = r.get("status").unwrap_or(&Value::Null);

            PlanRow {
                name: metadata
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                namespace: if show_namespace {
                    metadata
                        .get("namespace")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                },
                phase: status
                    .get("phase")
                    .and_then(|p| p.as_str())
                    .unwrap_or("-")
                    .to_string(),
                nodes: status
                    .get("nodeCount")
                    .and_then(|n| n.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or("-".to_string()),
                version: spec
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string(),
            }
        })
        .collect();

    Table::new(rows).to_string()
}

#[derive(Tabled)]
struct CapabilityRow {
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "NAMESPACE")]
    namespace: String,
    #[tabled(rename = "PHASE")]
    phase: String,
    #[tabled(rename = "CATEGORY")]
    category: String,
}

fn format_capabilities(resources: &[Value], show_namespace: bool) -> String {
    let rows: Vec<CapabilityRow> = resources
        .iter()
        .map(|r| {
            let metadata = r.get("metadata").unwrap_or(&Value::Null);
            let spec = r.get("spec").unwrap_or(&Value::Null);
            let status = r.get("status").unwrap_or(&Value::Null);

            CapabilityRow {
                name: metadata
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                namespace: if show_namespace {
                    metadata
                        .get("namespace")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                },
                phase: status
                    .get("phase")
                    .and_then(|p| p.as_str())
                    .unwrap_or("-")
                    .to_string(),
                category: spec
                    .get("category")
                    .and_then(|c| c.as_str())
                    .unwrap_or("-")
                    .to_string(),
            }
        })
        .collect();

    Table::new(rows).to_string()
}

#[derive(Tabled)]
struct BindingRow {
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "NAMESPACE")]
    namespace: String,
    #[tabled(rename = "PHASE")]
    phase: String,
    #[tabled(rename = "RULES")]
    rules: String,
}

fn format_bindings(resources: &[Value], show_namespace: bool) -> String {
    let rows: Vec<BindingRow> = resources
        .iter()
        .map(|r| {
            let metadata = r.get("metadata").unwrap_or(&Value::Null);
            let spec = r.get("spec").unwrap_or(&Value::Null);
            let status = r.get("status").unwrap_or(&Value::Null);

            BindingRow {
                name: metadata
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                namespace: if show_namespace {
                    metadata
                        .get("namespace")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                },
                phase: status
                    .get("phase")
                    .and_then(|p| p.as_str())
                    .unwrap_or("-")
                    .to_string(),
                rules: spec
                    .get("rules")
                    .and_then(|r| r.as_array())
                    .map(|r| r.len().to_string())
                    .unwrap_or("-".to_string()),
            }
        })
        .collect();

    Table::new(rows).to_string()
}

#[derive(Tabled)]
struct ExecutionRow {
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "NAMESPACE")]
    namespace: String,
    #[tabled(rename = "PHASE")]
    phase: String,
    #[tabled(rename = "PLAN")]
    plan: String,
}

fn format_executions(resources: &[Value], show_namespace: bool) -> String {
    let rows: Vec<ExecutionRow> = resources
        .iter()
        .map(|r| {
            let metadata = r.get("metadata").unwrap_or(&Value::Null);
            let spec = r.get("spec").unwrap_or(&Value::Null);
            let status = r.get("status").unwrap_or(&Value::Null);

            ExecutionRow {
                name: metadata
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                namespace: if show_namespace {
                    metadata
                        .get("namespace")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                },
                phase: status
                    .get("phase")
                    .and_then(|p| p.as_str())
                    .unwrap_or("-")
                    .to_string(),
                plan: spec
                    .get("planRef")
                    .and_then(|r| r.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("-")
                    .to_string(),
            }
        })
        .collect();

    Table::new(rows).to_string()
}
