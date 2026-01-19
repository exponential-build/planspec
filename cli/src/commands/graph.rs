use anyhow::Result;
use colored::Colorize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::client::{Client, Config};

/// Visualize a plan's DAG
pub async fn run(config: &Config, name: &str, format: &str, _output_format: &str) -> Result<()> {
    let client = Client::new(config)?;

    // Get the plan
    let plan = client.get("plans", name, None).await?;

    let graph = plan
        .get("spec")
        .and_then(|s| s.get("graph"))
        .ok_or_else(|| anyhow::anyhow!("Plan has no graph"))?;

    match format {
        "dot" => output_dot(name, graph),
        "mermaid" => output_mermaid(name, graph),
        _ => output_text(name, graph),
    }
}

fn output_text(name: &str, graph: &Value) -> Result<()> {
    let nodes = graph
        .get("nodes")
        .and_then(|n| n.as_array())
        .ok_or_else(|| anyhow::anyhow!("Graph has no nodes"))?;

    let empty_edges = vec![];
    let edges = graph
        .get("edges")
        .and_then(|e| e.as_array())
        .unwrap_or(&empty_edges);

    println!("{}", format!("Plan: {}", name).bold());
    println!("{}", "=".repeat(40));
    println!();

    // Build adjacency list
    let mut deps: HashMap<String, Vec<String>> = HashMap::new();
    let mut rdeps: HashMap<String, Vec<String>> = HashMap::new();

    for edge in edges {
        let from = edge.get("from").and_then(|f| f.as_str()).unwrap_or("");
        let to = edge.get("to").and_then(|t| t.as_str()).unwrap_or("");

        deps.entry(to.to_string())
            .or_default()
            .push(from.to_string());
        rdeps.entry(from.to_string())
            .or_default()
            .push(to.to_string());
    }

    // Find root nodes (no incoming edges)
    let node_ids: HashSet<String> = nodes
        .iter()
        .filter_map(|n| n.get("id").and_then(|i| i.as_str()).map(|s| s.to_string()))
        .collect();

    let root_nodes: Vec<&str> = node_ids
        .iter()
        .filter(|id| !deps.contains_key(*id) || deps.get(*id).unwrap().is_empty())
        .map(|s| s.as_str())
        .collect();

    // Build node map
    let node_map: HashMap<&str, &Value> = nodes
        .iter()
        .filter_map(|n| {
            n.get("id")
                .and_then(|i| i.as_str())
                .map(|id| (id, n))
        })
        .collect();

    // Print tree
    let mut visited = HashSet::new();

    for root in root_nodes {
        print_node(root, &node_map, &rdeps, &mut visited, 0);
    }

    // Print any remaining nodes (in case of complex graphs)
    for node in nodes {
        let id = node.get("id").and_then(|i| i.as_str()).unwrap_or("");
        if !visited.contains(id) {
            println!();
            println!("{}", "(disconnected)".dimmed());
            print_node(id, &node_map, &rdeps, &mut visited, 0);
        }
    }

    println!();
    println!(
        "{} {} nodes, {} edges",
        "Summary:".bold(),
        nodes.len(),
        edges.len()
    );

    Ok(())
}

fn print_node(
    id: &str,
    node_map: &HashMap<&str, &Value>,
    rdeps: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    depth: usize,
) {
    if visited.contains(id) {
        let indent = "  ".repeat(depth);
        println!("{}{}... (cycle)", indent, id.dimmed());
        return;
    }
    visited.insert(id.to_string());

    let node = node_map.get(id);
    let kind = node
        .and_then(|n| n.get("kind"))
        .and_then(|k| k.as_str())
        .unwrap_or("?");
    let desc = node
        .and_then(|n| n.get("description"))
        .and_then(|d| d.as_str())
        .unwrap_or("");

    let indent = "  ".repeat(depth);
    let prefix = if depth == 0 { "◆" } else { "├─" };

    let kind_colored = match kind {
        "Task" => kind.blue(),
        "Gate" => kind.yellow(),
        "Group" => kind.magenta(),
        "External" => kind.cyan(),
        _ => kind.normal(),
    };

    println!(
        "{}{} [{}] {} - {}",
        indent,
        prefix,
        kind_colored,
        id.cyan().bold(),
        desc.dimmed()
    );

    // Print children
    if let Some(children) = rdeps.get(id) {
        for (i, child) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;
            let _child_indent = if is_last { "└─" } else { "├─" };
            // Recursively print
            print_node(child, node_map, rdeps, visited, depth + 1);
        }
    }
}

fn output_dot(name: &str, graph: &Value) -> Result<()> {
    let nodes = graph
        .get("nodes")
        .and_then(|n| n.as_array())
        .ok_or_else(|| anyhow::anyhow!("Graph has no nodes"))?;

    let empty_edges = vec![];
    let edges = graph
        .get("edges")
        .and_then(|e| e.as_array())
        .unwrap_or(&empty_edges);

    println!("digraph {} {{", name.replace('-', "_"));
    println!("  rankdir=TB;");
    println!("  node [shape=box, style=rounded];");
    println!();

    // Output nodes
    for node in nodes {
        let id = node.get("id").and_then(|i| i.as_str()).unwrap_or("");
        let kind = node.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        let desc = node
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");

        let shape = match kind {
            "Gate" => "diamond",
            "Group" => "box3d",
            "External" => "parallelogram",
            _ => "box",
        };

        let color = match kind {
            "Task" => "lightblue",
            "Gate" => "lightyellow",
            "Group" => "lightgreen",
            "External" => "lightgray",
            _ => "white",
        };

        println!(
            "  {} [label=\"{}\\n{}\", shape={}, fillcolor={}, style=\"rounded,filled\"];",
            id.replace('-', "_"),
            id,
            truncate(desc, 30),
            shape,
            color
        );
    }

    println!();

    // Output edges
    for edge in edges {
        let from = edge.get("from").and_then(|f| f.as_str()).unwrap_or("");
        let to = edge.get("to").and_then(|t| t.as_str()).unwrap_or("");
        let edge_type = edge
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("hard");

        let style = if edge_type == "soft" { "dashed" } else { "solid" };

        println!(
            "  {} -> {} [style={}];",
            from.replace('-', "_"),
            to.replace('-', "_"),
            style
        );
    }

    println!("}}");

    Ok(())
}

fn output_mermaid(name: &str, graph: &Value) -> Result<()> {
    let nodes = graph
        .get("nodes")
        .and_then(|n| n.as_array())
        .ok_or_else(|| anyhow::anyhow!("Graph has no nodes"))?;

    let empty_edges = vec![];
    let edges = graph
        .get("edges")
        .and_then(|e| e.as_array())
        .unwrap_or(&empty_edges);

    println!("```mermaid");
    println!("flowchart TD");
    println!("    %% Plan: {}", name);
    println!();

    // Output nodes
    for node in nodes {
        let id = node.get("id").and_then(|i| i.as_str()).unwrap_or("");
        let kind = node.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        let desc = node
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");

        let mermaid_id = id.replace('-', "_");
        let label = format!("{}: {}", id, truncate(desc, 25));

        match kind {
            "Gate" => println!("    {}{{\"{}\"}}",  mermaid_id, label),
            "Group" => println!("    {}[[\"{}\"]]]", mermaid_id, label),
            "External" => println!("    {}[/\"{}\"/]", mermaid_id, label),
            _ => println!("    {}[\"{}\"]", mermaid_id, label),
        }
    }

    println!();

    // Output edges
    for edge in edges {
        let from = edge.get("from").and_then(|f| f.as_str()).unwrap_or("");
        let to = edge.get("to").and_then(|t| t.as_str()).unwrap_or("");
        let edge_type = edge
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("hard");

        let from_id = from.replace('-', "_");
        let to_id = to.replace('-', "_");

        if edge_type == "soft" {
            println!("    {} -.-> {}", from_id, to_id);
        } else {
            println!("    {} --> {}", from_id, to_id);
        }
    }

    println!("```");

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
