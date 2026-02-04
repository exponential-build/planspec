use anyhow::{bail, Result};

use crate::client::{Client, Config};

/// Check if resource type is a namespace
fn is_namespace(resource: &str) -> bool {
    matches!(
        resource.to_lowercase().as_str(),
        "namespace" | "namespaces" | "ns"
    )
}

pub async fn run(config: &Config, resource: &str, name: &str) -> Result<()> {
    let client = Client::new(config)?;

    // Currently only namespace creation is supported via create command
    if is_namespace(resource) {
        client.create_namespace(name).await?;
        println!("namespace \"{}\" created", name);
    } else {
        bail!("create command only supports 'namespace' resource type. Use 'planspec apply -f <file>' for other resources.");
    }

    Ok(())
}
