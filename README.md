# PlanSpec

**Open source infrastructure for declarative work orchestration**

Developed by [Exponential Build, Inc.](https://exponential.build)

---

## Overview

PlanSpec provides a Kubernetes-inspired control plane for expressing goals, plans, and executions. It defines the core resource contracts for orchestrating work across humans and AI agents.

PlanSpec is intentionally minimal and implementation-agnostic. It defines the contracts; implementations may build richer features (knowledge graphs, retrieval, learning) on top.

### Core Primitives

| Primitive | Scope | Description |
|-----------|-------|-------------|
| **Goal** | Namespaced | High-level objective with acceptance criteria ("what") |
| **Plan** | Namespaced | DAG of work with embedded nodes/edges ("how") |
| **Capability** | Namespaced | Reusable ability that agents can provide |
| **Binding** | Namespaced | Resolution of capability requirements to provider targets |
| **Execution** | Namespaced | Intent to run a Plan with specific bindings |

### Key Principles

- **Declarative over imperative**: Describe what you want, not how to get there
- **Plan is the durable artifact**: Nodes are embedded in `Plan.spec.graph`, not separate resources
- **Late binding**: Capabilities resolved at execution time via Bindings
- **Immutable intent**: `Execution.spec` is immutable; `Execution.status` tracks runs
- **Reconciliation-driven**: Controllers continuously drive toward desired state

## Quick Start

### Installation

```bash
# Build from source
cd cli
cargo build --release

# Or install directly
cargo install --path cli
```

### Validate a Specification

```bash
planspec validate -f goal.yaml
```

### Apply Resources

```bash
planspec apply -f goal.yaml
```

### Watch Executions

```bash
planspec watch executions
```

### Visualize a Plan

```bash
planspec graph sso-implementation-v2
```

## Example

### Goal

```yaml
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: enable-google-sso
  namespace: team-platform
spec:
  description: "Enable Google SSO for the application"
  constraints:
    frontend: react
    idp: google
  acceptanceCriteria:
    - description: "Users can sign in with Google"
    - description: "Existing sessions remain valid"
  planSelector:
    matchLabels:
      planspec.io/goal: enable-google-sso
  timeout: "7d"
  priority: 10
```

### Plan

```yaml
apiVersion: planspec.io/v1alpha1
kind: Plan
metadata:
  name: sso-implementation-v2
  namespace: team-platform
  labels:
    planspec.io/goal: enable-google-sso
    planspec.io/series: sso
    planspec.io/version: "2"
spec:
  goalRef:
    name: enable-google-sso
  series: sso
  version: "2"
  description: "Implementation plan for Google SSO"
  graph:
    nodes:
      - id: research
        kind: Task
        description: "Research OAuth2/OIDC requirements"
        capabilityRef:
          name: code-analysis
      - id: implement
        kind: Task
        description: "Implement SSO flow"
        capabilityRef:
          name: code-generation
        inputs:
          framework: react
        timeout: "4h"
        retries: 2
      - id: review
        kind: Gate
        description: "Security review checkpoint"
    edges:
      - from: research
        to: implement
      - from: implement
        to: review
```

### Execution

```yaml
apiVersion: planspec.io/v1alpha1
kind: Execution
metadata:
  name: sso-exec-001
  namespace: team-platform
spec:
  goalRef:
    name: enable-google-sso
  planRef:
    name: sso-implementation-v2
  bindingRef:
    name: platform-bindings
  parameters:
    dryRun: false
    priority: high
```

## Documentation

- [SPECIFICATION.md](./SPECIFICATION.md) - Formal definitions of all primitives
- [ARCHITECTURE.md](./ARCHITECTURE.md) - System design and reconciliation model
- [CONTRIBUTING.md](./CONTRIBUTING.md) - Development setup and contribution guidelines
- [Examples](./docs/examples/) - Sample specifications for common scenarios

## Project Structure

```
planspec/
├── api/openapi/          # OpenAPI specification
├── cli/                  # Rust CLI tool
├── docs/examples/        # Example specifications
└── schemas/              # JSON Schema definitions
    ├── common/           # Shared schemas (metadata, status, graph)
    └── v1alpha1/         # Version-specific resource schemas
```

## API Version

PlanSpec follows Kubernetes API versioning conventions:

- `v1alpha1` - Current version, breaking changes expected
- `v1beta1` - (Future) More stable, deprecation policy applies
- `v1` - (Future) Stable, backwards-compatible changes only

## License

MIT
