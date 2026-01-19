use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

use super::Config;

/// HTTP client for the PlanSpec API
#[allow(dead_code)]
pub struct Client {
    http: reqwest::Client,
    config: Config,
}

#[allow(dead_code)]
impl Client {
    pub fn new(config: &Config) -> Result<Self> {
        let http = reqwest::Client::builder()
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            http,
            config: config.clone(),
        })
    }

    fn base_url(&self) -> String {
        format!(
            "{}/apis/planspec.io/v1alpha1",
            self.config.server_url()
        )
    }

    /// List resources of a given type
    pub async fn list(
        &self,
        resource: &str,
        namespace: Option<&str>,
        selector: Option<&str>,
        all_namespaces: bool,
    ) -> Result<Value> {
        let default_ns = self.config.namespace();
        let url = if all_namespaces {
            format!("{}/{}?allNamespaces=true", self.base_url(), resource)
        } else {
            let ns = namespace.unwrap_or(&default_ns);
            format!("{}/namespaces/{}/{}", self.base_url(), ns, resource)
        };

        let mut req = self.http.get(&url);
        if let Some(sel) = selector {
            req = req.query(&[("labelSelector", sel)]);
        }

        let response = req.send().await.context("Failed to send request")?;
        self.handle_response(response).await
    }

    /// Get a single resource
    pub async fn get(&self, resource: &str, name: &str, namespace: Option<&str>) -> Result<Value> {
        let default_ns = self.config.namespace();
        let ns = namespace.unwrap_or(&default_ns);
        let url = format!(
            "{}/namespaces/{}/{}/{}",
            self.base_url(),
            ns,
            resource,
            name
        );

        let response = self.http.get(&url).send().await.context("Failed to send request")?;
        self.handle_response(response).await
    }

    /// Create a resource
    pub async fn create<T: Serialize>(&self, resource: &str, namespace: &str, body: &T) -> Result<Value> {
        let url = format!(
            "{}/namespaces/{}/{}",
            self.base_url(),
            namespace,
            resource
        );

        let response = self
            .http
            .post(&url)
            .json(body)
            .send()
            .await
            .context("Failed to send request")?;

        self.handle_response(response).await
    }

    /// Update a resource
    pub async fn update<T: Serialize>(
        &self,
        resource: &str,
        name: &str,
        namespace: &str,
        body: &T,
    ) -> Result<Value> {
        let url = format!(
            "{}/namespaces/{}/{}/{}",
            self.base_url(),
            namespace,
            resource,
            name
        );

        let response = self
            .http
            .put(&url)
            .json(body)
            .send()
            .await
            .context("Failed to send request")?;

        self.handle_response(response).await
    }

    /// Delete a resource
    pub async fn delete(&self, resource: &str, name: &str, namespace: Option<&str>) -> Result<Value> {
        let default_ns = self.config.namespace();
        let ns = namespace.unwrap_or(&default_ns);
        let url = format!(
            "{}/namespaces/{}/{}/{}",
            self.base_url(),
            ns,
            resource,
            name
        );

        let response = self.http.delete(&url).send().await.context("Failed to send request")?;
        self.handle_response(response).await
    }

    /// Apply multiple resources
    pub async fn apply(&self, namespace: &str, resources: &[Value]) -> Result<Value> {
        let url = format!("{}/namespaces/{}/apply", self.base_url(), namespace);

        let response = self
            .http
            .post(&url)
            .json(resources)
            .send()
            .await
            .context("Failed to send request")?;

        self.handle_response(response).await
    }

    /// Validate resources without persisting
    pub async fn validate(&self, resources: &[Value]) -> Result<Value> {
        let url = format!("{}/validate", self.base_url());

        let response = self
            .http
            .post(&url)
            .json(resources)
            .send()
            .await
            .context("Failed to send request")?;

        self.handle_response(response).await
    }

    /// Get a plan's graph
    pub async fn get_graph(&self, name: &str, namespace: Option<&str>) -> Result<Value> {
        let default_ns = self.config.namespace();
        let ns = namespace.unwrap_or(&default_ns);
        let url = format!(
            "{}/namespaces/{}/plans/{}/graph",
            self.base_url(),
            ns,
            name
        );

        let response = self.http.get(&url).send().await.context("Failed to send request")?;
        self.handle_response(response).await
    }

    async fn handle_response(&self, response: reqwest::Response) -> Result<Value> {
        let status = response.status();
        let body = response.text().await.context("Failed to read response body")?;

        if status.is_success() {
            serde_json::from_str(&body).context("Failed to parse response JSON")
        } else {
            let error: Value = serde_json::from_str(&body).unwrap_or_else(|_| {
                serde_json::json!({
                    "status": "Failure",
                    "message": body,
                    "code": status.as_u16()
                })
            });

            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");

            anyhow::bail!("{}: {}", status, message)
        }
    }
}
