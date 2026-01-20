use anyhow::Result;

use crate::client::{Client, Config};

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

pub async fn run(config: &Config, resource: &str, name: &str) -> Result<()> {
    let client = Client::new(config)?;

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
