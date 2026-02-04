use std::path::PathBuf;

use anyhow::Result;
use planspec_server::{run_server, ServerConfig};

/// Get the default database path using platform-appropriate data directory.
///
/// - Linux: `$XDG_DATA_HOME/planspec/planspec.db` (defaults to `~/.local/share/planspec/planspec.db`)
/// - macOS: `~/Library/Application Support/planspec/planspec.db`
/// - Windows: `%APPDATA%\planspec\planspec.db`
///
/// Falls back to `planspec.db` in the current directory if the data directory cannot be determined.
pub fn default_db_path() -> PathBuf {
    dirs::data_dir()
        .map(|p| p.join("planspec").join("planspec.db"))
        .unwrap_or_else(|| PathBuf::from("planspec.db"))
}

pub async fn run(host: &str, port: u16, db: &str, reconcile_interval: u64) -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(db).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let config = ServerConfig {
        host: host.to_string(),
        port,
        db_path: db.to_string(),
        reconcile_interval_secs: reconcile_interval,
    };

    run_server(config).await
}
