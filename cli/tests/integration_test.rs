//! Integration tests for the PlanSpec CLI
//!
//! These tests require a running PlanSpec server at localhost:8080.
//! Run with: cargo test --test integration_test -- --test-threads=1

use std::process::Command;
use std::time::Duration;

const CLI_PATH: &str = env!("CARGO_BIN_EXE_planspec");
const SERVER_URL: &str = "http://localhost:8080";
const TEST_NAMESPACE: &str = "test-integration";

/// Check if server is running
fn server_is_running() -> bool {
    reqwest::blocking::Client::new()
        .get(format!("{}/health", SERVER_URL))
        .timeout(Duration::from_secs(2))
        .send()
        .is_ok()
}

/// Run planspec CLI command and return output
fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(CLI_PATH)
        .args(args)
        .output()
        .expect("Failed to execute CLI")
}

/// Run planspec CLI and return stdout as string
fn run_cli_stdout(args: &[&str]) -> String {
    let output = run_cli(args);
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Run planspec CLI and check success
fn run_cli_success(args: &[&str]) -> bool {
    run_cli(args).status.success()
}

mod server_required {
    use super::*;

    fn skip_if_no_server() {
        if !server_is_running() {
            eprintln!("Skipping test: server not running at {}", SERVER_URL);
            return;
        }
    }

    #[test]
    fn test_apply_goal() {
        skip_if_no_server();
        if !server_is_running() {
            return;
        }

        // Create a test goal
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

        // Write to temp file
        let temp_file = std::env::temp_dir().join("test-goal.yaml");
        std::fs::write(&temp_file, goal_yaml).expect("Failed to write temp file");

        // Apply
        let output = run_cli(&["apply", "-f", temp_file.to_str().unwrap()]);
        assert!(output.status.success(), "Apply should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("test-goal-apply") && stdout.contains("created"),
            "Should report goal created: {}",
            stdout
        );

        // Cleanup
        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_get_goals() {
        skip_if_no_server();
        if !server_is_running() {
            return;
        }

        let output = run_cli(&["get", "goals", "-n", TEST_NAMESPACE]);
        assert!(output.status.success(), "Get goals should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should show table header or "No resources found"
        assert!(
            stdout.contains("NAME") || stdout.contains("No resources"),
            "Should show goals table or empty message"
        );
    }

    #[test]
    fn test_apply_plan() {
        skip_if_no_server();
        if !server_is_running() {
            return;
        }

        let plan_yaml = r#"
apiVersion: planspec.io/v1alpha1
kind: Plan
metadata:
  name: test-plan-apply-v1
  namespace: test-integration
  labels:
    planspec.io/goal: test-goal-apply
    planspec.io/series: test-plan-apply
    planspec.io/version: "1"
spec:
  goalRef:
    name: test-goal-apply
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
        assert!(output.status.success(), "Apply plan should succeed");

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
        skip_if_no_server();
        if !server_is_running() {
            return;
        }

        let output = run_cli(&["get", "plans", "-n", TEST_NAMESPACE]);
        assert!(output.status.success(), "Get plans should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("NAME") || stdout.contains("No resources"),
            "Should show plans table or empty message"
        );
    }

    #[test]
    fn test_describe_goal() {
        skip_if_no_server();
        if !server_is_running() {
            return;
        }

        // First ensure the goal exists
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

        // Now describe it
        let output = run_cli(&["describe", "goal", "test-describe-goal", "-n", TEST_NAMESPACE]);
        assert!(output.status.success(), "Describe should succeed");

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
        skip_if_no_server();
        if !server_is_running() {
            return;
        }

        // Test applying the fixture bundle
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
        skip_if_no_server();
        if !server_is_running() {
            return;
        }

        let output = run_cli(&["get", "goals", "-A"]);
        assert!(output.status.success(), "Get with -A should succeed");
    }

    #[test]
    fn test_yaml_output() {
        skip_if_no_server();
        if !server_is_running() {
            return;
        }

        let output = run_cli(&["get", "goals", "-n", "planspec", "-o", "yaml"]);
        assert!(output.status.success(), "Get with YAML output should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should be valid YAML (starts with - or apiVersion or empty)
        assert!(
            stdout.is_empty()
                || stdout.starts_with('-')
                || stdout.starts_with("apiVersion")
                || stdout.contains("items:"),
            "Should produce YAML output: {}",
            &stdout[..stdout.len().min(200)]
        );
    }
}

mod offline {
    use super::*;

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

        let output = run_cli(&["validate", "-f", temp_file.to_str().unwrap()]);
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

        let output = run_cli(&["validate", "-f", temp_file.to_str().unwrap()]);
        // Should fail validation
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

        let output = run_cli(&["validate", "-f", bundle_path]);
        assert!(
            output.status.success(),
            "Bundle validation should pass: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_help() {
        let output = run_cli(&["--help"]);
        assert!(output.status.success(), "Help should work");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("planspec"), "Should show planspec in help");
    }

    #[test]
    fn test_version() {
        let output = run_cli(&["--version"]);
        assert!(output.status.success(), "Version should work");
    }
}
