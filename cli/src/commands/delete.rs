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

pub async fn run(config: &Config, resource: &str, name: &str) -> Result<()> {
    let client = Client::new(config)?;
    let resource_type = normalize_resource_type(resource);

    client.delete(resource_type, name, None).await?;

    println!("{} \"{}\" deleted", resource, name);

    Ok(())
}
