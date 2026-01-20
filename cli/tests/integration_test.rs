//! Integration tests for the PlanSpec CLI
//!
//! These tests automatically start an ephemeral server with an in-memory database.
//! Run with: cargo test --test integration_test
//!
//! Set PLANSPEC_SERVER_BIN to override the server binary path.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

const CLI_PATH: &str = env!("CARGO_BIN_EXE_planspec");
const TEST_NAMESPACE: &str = "test-integration";

/// Global test server instance
static TEST_SERVER: OnceLock<TestServer> = OnceLock::new();

/// A test server running with in-memory SQLite
struct TestServer {
    _process: Child,
    pub port: u16,
}

impl TestServer {
    /// Start a new test server on a random port with in-memory database
    fn start() -> Self {
        // Find the server binary - check env var first, then look relative to CLI
        let server_path = if let Ok(path) = std::env::var("PLANSPEC_SERVER_BIN") {
            std::path::PathBuf::from(path)
        } else {
            std::path::Path::new(CLI_PATH)
                .parent()
                .unwrap()
                .join("planspec-server")
        };

        if !server_path.exists() {
            panic!(
                "Server binary not found at {:?}. Build with: cargo build -p planspec-server",
                server_path
            );
        }

        // Find an available port
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        // Start server with in-memory database
        let mut process = Command::new(&server_path)
            .env("PLANSPEC_DB", ":memory:")
            .env("PORT", port.to_string())
            .env("RUST_LOG", "warn") // Quiet logging
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start test server");

        // Wait for server to be ready by checking stderr for the listening message
        let stderr = process.stderr.take().unwrap();
        let reader = BufReader::new(stderr);

        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(10);

        // Spawn a thread to read stderr and look for ready message
        let port_check = port;
        let ready = std::thread::spawn(move || {
            for line in reader.lines() {
                if let Ok(line) = line {
                    if line.contains("listening") {
                        return true;
                    }
                }
                if start.elapsed() > timeout {
                    return false;
                }
            }
            false
        });

        // Also poll the health endpoint
        let client = reqwest::blocking::Client::new();
        let health_url = format!("http://127.0.0.1:{}/healthz", port_check);

        for _ in 0..50 {
            if client
                .get(&health_url)
                .timeout(Duration::from_millis(100))
                .send()
                .is_ok()
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        // Verify server is responding
        let response = client
            .get(&health_url)
            .timeout(Duration::from_secs(2))
            .send();

        if response.is_err() {
            let _ = ready.join();
            panic!("Server failed to start on port {}", port);
        }

        TestServer {
            _process: process,
            port,
        }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

/// Get or initialize the test server
fn get_server() -> &'static TestServer {
    TEST_SERVER.get_or_init(TestServer::start)
}

/// Run planspec CLI command against the test server
fn run_cli(args: &[&str]) -> std::process::Output {
    let server = get_server();
    Command::new(CLI_PATH)
        .args(args)
        .env("PLANSPEC_SERVER", server.url())
        .output()
        .expect("Failed to execute CLI")
}

/// Run planspec CLI and return stdout as string
#[allow(dead_code)]
fn run_cli_stdout(args: &[&str]) -> String {
    let output = run_cli(args);
    String::from_utf8_lossy(&output.stdout).to_string()
}

mod server_tests {
    use super::*;

    #[test]
    fn test_apply_goal() {
        let goal_yaml = r#"
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: test-goal-apply
  namespace: test-integration
spec:
  description: "Test goal for integration tests"
  acceptanceCriteria:
    - description: "Test criterion"
  priority: 50
"#;

        let temp_file = std::env::temp_dir().join("test-goal.yaml");
        std::fs::write(&temp_file, goal_yaml).expect("Failed to write temp file");

        let output = run_cli(&["apply", "-f", temp_file.to_str().unwrap()]);
        assert!(
            output.status.success(),
            "Apply should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("test-goal-apply")
                && (stdout.contains("created") || stdout.contains("configured")),
            "Should report goal created or configured: {}",
            stdout
        );

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_get_goals() {
        // First apply a goal to ensure there's something to get
        let goal_yaml = r#"
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: test-goal-get
  namespace: test-integration
spec:
  description: "Goal for get test"
  acceptanceCriteria:
    - description: "Criterion"
  priority: 50
"#;

        let temp_file = std::env::temp_dir().join("test-goal-get.yaml");
        std::fs::write(&temp_file, goal_yaml).expect("Failed to write temp file");
        run_cli(&["apply", "-f", temp_file.to_str().unwrap()]);

        let output = run_cli(&["get", "goals", "-n", TEST_NAMESPACE]);
        assert!(output.status.success(), "Get goals should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("NAME") || stdout.contains("test-goal"),
            "Should show goals table: {}",
            stdout
        );

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_apply_plan() {
        // First ensure the goal exists
        let goal_yaml = r#"
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: test-goal-for-plan
  namespace: test-integration
spec:
  description: "Goal for plan test"
  acceptanceCriteria:
    - description: "Criterion"
  priority: 50
"#;

        let temp_file = std::env::temp_dir().join("test-goal-for-plan.yaml");
        std::fs::write(&temp_file, goal_yaml).expect("Failed to write temp file");
        run_cli(&["apply", "-f", temp_file.to_str().unwrap()]);

        let plan_yaml = r#"
apiVersion: planspec.io/v1alpha1
kind: Plan
metadata:
  name: test-plan-apply-v1
  namespace: test-integration
  labels:
    planspec.io/goal: test-goal-for-plan
    planspec.io/series: test-plan-apply
    planspec.io/version: "1"
spec:
  goalRef:
    name: test-goal-for-plan
  series: test-plan-apply
  version: "1"
  description: "Test plan for integration tests"
  graph:
    nodes:
      - id: step-1
        kind: Task
        description: "First step"
      - id: step-2
        kind: Task
        description: "Second step"
    edges:
      - from: step-1
        to: step-2
"#;

        let temp_file = std::env::temp_dir().join("test-plan.yaml");
        std::fs::write(&temp_file, plan_yaml).expect("Failed to write temp file");

        let output = run_cli(&["apply", "-f", temp_file.to_str().unwrap()]);
        assert!(
            output.status.success(),
            "Apply plan should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("test-plan-apply-v1"),
            "Should report plan created: {}",
            stdout
        );

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_get_plans() {
        let output = run_cli(&["get", "plans", "-n", TEST_NAMESPACE]);
        assert!(output.status.success(), "Get plans should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("NAME") || stdout.contains("No resources"),
            "Should show plans table or empty message: {}",
            stdout
        );
    }

    #[test]
    fn test_describe_goal() {
        let goal_yaml = r#"
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: test-describe-goal
  namespace: test-integration
spec:
  description: "Goal for describe test"
  acceptanceCriteria:
    - description: "Criterion 1"
  priority: 100
"#;

        let temp_file = std::env::temp_dir().join("test-describe-goal.yaml");
        std::fs::write(&temp_file, goal_yaml).expect("Failed to write temp file");
        run_cli(&["apply", "-f", temp_file.to_str().unwrap()]);

        let output = run_cli(&[
            "describe",
            "goal",
            "test-describe-goal",
            "-n",
            TEST_NAMESPACE,
        ]);
        assert!(
            output.status.success(),
            "Describe should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("test-describe-goal") && stdout.contains("Goal for describe test"),
            "Should show goal details: {}",
            stdout
        );

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_apply_bundle() {
        let bundle_path = "docs/planspec/fixtures/v0.1.0/bundle.yaml";
        if !std::path::Path::new(bundle_path).exists() {
            eprintln!("Skipping bundle test: {} not found", bundle_path);
            return;
        }

        let output = run_cli(&["apply", "-f", bundle_path]);
        assert!(
            output.status.success(),
            "Apply bundle should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_get_all_namespaces() {
        let output = run_cli(&["get", "goals", "-A"]);
        assert!(
            output.status.success(),
            "Get with -A should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_yaml_output() {
        // First apply something
        let goal_yaml = r#"
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: test-yaml-output
  namespace: test-integration
spec:
  description: "Goal for YAML output test"
  acceptanceCriteria:
    - description: "Criterion"
  priority: 50
"#;

        let temp_file = std::env::temp_dir().join("test-yaml-output.yaml");
        std::fs::write(&temp_file, goal_yaml).expect("Failed to write temp file");
        run_cli(&["apply", "-f", temp_file.to_str().unwrap()]);

        let output = run_cli(&["get", "goals", "-n", TEST_NAMESPACE, "-o", "yaml"]);
        assert!(
            output.status.success(),
            "Get with YAML output should succeed"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("apiVersion") || stdout.contains("items"),
            "Should produce YAML output: {}",
            &stdout[..stdout.len().min(500)]
        );

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_delete_goal() {
        // First create a goal
        let goal_yaml = r#"
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: test-delete-goal
  namespace: test-integration
spec:
  description: "Goal to be deleted"
  acceptanceCriteria:
    - description: "Criterion"
  priority: 50
"#;

        let temp_file = std::env::temp_dir().join("test-delete-goal.yaml");
        std::fs::write(&temp_file, goal_yaml).expect("Failed to write temp file");
        run_cli(&["apply", "-f", temp_file.to_str().unwrap()]);

        // Delete it
        let output = run_cli(&["delete", "goal", "test-delete-goal", "-n", TEST_NAMESPACE]);
        assert!(
            output.status.success(),
            "Delete should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify it's gone
        let output = run_cli(&["describe", "goal", "test-delete-goal", "-n", TEST_NAMESPACE]);
        assert!(
            !output.status.success()
                || String::from_utf8_lossy(&output.stderr).contains("not found"),
            "Goal should be deleted"
        );

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_list_namespaces() {
        // Apply something first to create a namespace
        let goal_yaml = r#"
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: test-ns-goal
  namespace: test-ns-list
spec:
  description: "Goal in test namespace"
  acceptanceCriteria:
    - description: "Criterion"
  priority: 50
"#;

        let temp_file = std::env::temp_dir().join("test-ns-goal.yaml");
        std::fs::write(&temp_file, goal_yaml).expect("Failed to write temp file");
        run_cli(&["apply", "-f", temp_file.to_str().unwrap()]);

        let output = run_cli(&["get", "namespaces"]);
        assert!(
            output.status.success(),
            "Get namespaces should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("test-ns-list"),
            "Should list the namespace: {}",
            stdout
        );

        std::fs::remove_file(temp_file).ok();
    }
}

mod offline {
    use super::*;

    /// Run CLI without server (offline validation)
    fn run_cli_offline(args: &[&str]) -> std::process::Output {
        Command::new(CLI_PATH)
            .args(args)
            .output()
            .expect("Failed to execute CLI")
    }

    #[test]
    fn test_validate_valid_goal() {
        let goal_yaml = r#"
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: valid-goal
  namespace: default
spec:
  description: "A valid goal"
  acceptanceCriteria:
    - description: "Criterion"
  priority: 100
"#;

        let temp_file = std::env::temp_dir().join("valid-goal.yaml");
        std::fs::write(&temp_file, goal_yaml).expect("Failed to write temp file");

        let output = run_cli_offline(&["validate", "-f", temp_file.to_str().unwrap()]);
        assert!(output.status.success(), "Validation should pass");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("valid"), "Should report valid");

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_validate_invalid_goal() {
        let invalid_yaml = r#"
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: invalid-goal
spec:
  # Missing required fields
"#;

        let temp_file = std::env::temp_dir().join("invalid-goal.yaml");
        std::fs::write(&temp_file, invalid_yaml).expect("Failed to write temp file");

        let output = run_cli_offline(&["validate", "-f", temp_file.to_str().unwrap()]);
        assert!(
            !output.status.success() || String::from_utf8_lossy(&output.stderr).contains("error"),
            "Validation should fail for invalid goal"
        );

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_validate_bundle() {
        let bundle_path = "docs/planspec/fixtures/v0.1.0/bundle.yaml";
        if !std::path::Path::new(bundle_path).exists() {
            eprintln!("Skipping: {} not found", bundle_path);
            return;
        }

        let output = run_cli_offline(&["validate", "-f", bundle_path]);
        assert!(
            output.status.success(),
            "Bundle validation should pass: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_help() {
        let output = run_cli_offline(&["--help"]);
        assert!(output.status.success(), "Help should work");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("planspec"), "Should show planspec in help");
    }

    #[test]
    fn test_version() {
        let output = run_cli_offline(&["--version"]);
        assert!(output.status.success(), "Version should work");
    }
}
