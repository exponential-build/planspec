# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 - 2026-02-04

### Added

- Initial release of PlanSpec
- Core primitives: Goal, Plan, Capability, Binding, Execution
- JSON Schema validation for all resource types
- CLI tool (`planspec`) with commands:
  - `apply` - Apply resources from files or directories
  - `get` - List resources
  - `describe` - Show detailed resource information
  - `delete` - Remove resources
  - `validate` - Offline schema validation
  - `watch` - Watch for resource changes
  - `diff` - Compare local files with server state
  - `graph` - Visualize plan DAGs (text, DOT, Mermaid formats)
  - `edit` - Edit resources in your editor
  - `serve` - Embedded API server (optional feature)
  - `completions` - Shell completion generation
- Reference API server with SQLite storage
- Plan reconciliation with automatic status updates
- Capability resolution via Bindings
