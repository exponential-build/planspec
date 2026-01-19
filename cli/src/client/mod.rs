pub mod api;

pub use api::Client;

/// Client configuration
#[derive(Clone, Debug)]
pub struct Config {
    pub server: Option<String>,
    pub namespace: Option<String>,
}

impl Config {
    pub fn server_url(&self) -> String {
        self.server
            .clone()
            .unwrap_or_else(|| "http://localhost:8080".to_string())
    }

    pub fn namespace(&self) -> String {
        self.namespace.clone().unwrap_or_else(|| "default".to_string())
    }
}
