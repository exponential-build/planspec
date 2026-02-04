# Contributing to PlanSpec

Thank you for your interest in contributing to PlanSpec! This document provides guidelines for contributing to the project.

---

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Setup](#development-setup)
4. [Project Structure](#project-structure)
5. [Making Changes](#making-changes)
6. [Pull Request Process](#pull-request-process)
7. [Style Guidelines](#style-guidelines)
8. [Testing](#testing)
9. [Release Process](#release-process)

---

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). By participating, you agree to uphold this code.

---

## Getting Started

### Prerequisites

- **Rust** 1.75+ (for CLI development)
- **Node.js** 18+ (for schema tooling)
- **Git** 2.30+

### Quick Start

```bash
# Clone the repository
git clone https://github.com/exponential-build/planspec.git
cd planspec

# Build the CLI
cd cli
cargo build

# Run tests
cargo test

# Validate schemas
cd ../schemas
npm install
npm run validate
```

---

## Development Setup

### Rust CLI

```bash
cd cli

# Install dependencies and build
cargo build

# Run in development mode
cargo run -- validate -f ../docs/examples/simple-goal.yaml

# Run tests
cargo test

# Run with verbose output
RUST_LOG=debug cargo run -- validate -f example.yaml

# Format code
cargo fmt

# Run linter
cargo clippy
```

### JSON Schemas

```bash
cd schemas

# Install dependencies
npm install

# Validate all schemas
npm run validate

# Test example files against schemas
npm run test

# Generate bundle
npm run bundle
```

### OpenAPI Specification

```bash
cd api/openapi

# Validate OpenAPI spec
npx @redocly/cli lint v1alpha1.yaml

# Generate documentation
npx @redocly/cli build-docs v1alpha1.yaml
```

---

## Project Structure

```
planspec/
├── README.md                    # Project overview
├── SPECIFICATION.md             # Formal specification
├── ARCHITECTURE.md              # System design
├── CONTRIBUTING.md              # This file
├── LICENSE                      # MIT
│
├── docs/
│   └── examples/                # Example specifications
│       ├── simple-goal.yaml
│       ├── sso-integration.yaml
│       └── capability-lib.yaml
│
├── schemas/                     # JSON Schema definitions
│   ├── common/                  # Shared schemas
│   │   ├── metadata.schema.json
│   │   ├── reference.schema.json
│   │   ├── status.schema.json
│   │   └── graph.schema.json
│   ├── v1alpha1/                # Version-specific schemas
│   │   ├── goal.schema.json
│   │   ├── plan.schema.json
│   │   ├── capability.schema.json
│   │   ├── binding.schema.json
│   │   ├── execution.schema.json
│   │   └── document.schema.json
│   └── conformance/rbac/        # Optional RBAC schemas
│
├── api/
│   └── openapi/
│       └── v1alpha1.yaml        # OpenAPI specification
│
└── cli/                         # Rust CLI
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── commands/            # CLI commands
        ├── client/              # API client
        ├── schema/              # Schema validation
        └── output/              # Output formatting
```

---

## Making Changes

### Branching Strategy

- `main` — stable, release-ready code
- `feature/*` — new features
- `fix/*` — bug fixes
- `docs/*` — documentation updates

### Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting (no code change)
- `refactor`: Code restructuring
- `test`: Tests
- `chore`: Maintenance

Examples:
```
feat(cli): add graph visualization command
fix(schema): correct Plan node reference validation
docs: update SPECIFICATION.md with Graph Views section
```

### Types of Contributions

| Contribution | Where to Start |
|--------------|----------------|
| Bug reports | Open an issue with reproduction steps |
| Feature requests | Open an issue describing the use case |
| Documentation | Edit markdown files, submit PR |
| Schema changes | Modify schemas, update tests |
| CLI features | Add commands in `cli/src/commands/` |

---

## Pull Request Process

### Before Submitting

1. **Create an issue** first for significant changes
2. **Fork the repository** and create a branch
3. **Make your changes** following style guidelines
4. **Add tests** for new functionality
5. **Update documentation** if needed
6. **Run all checks** locally:

```bash
# Rust
cd cli
cargo fmt --check
cargo clippy -- -D warnings
cargo test

# Schemas
cd schemas
npm run validate
npm run test
```

### PR Template

```markdown
## Description
Brief description of changes.

## Related Issue
Fixes #123

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Checklist
- [ ] Tests pass locally
- [ ] Code follows style guidelines
- [ ] Documentation updated
- [ ] Commit messages follow convention
```

### Review Process

1. **Automated checks** run on PR creation
2. **Maintainer review** within 3 business days
3. **Address feedback** with additional commits
4. **Squash and merge** when approved

---

## Style Guidelines

### Rust

Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/):

```rust
// Use descriptive names
fn validate_plan_graph(plan: &Plan) -> Result<(), ValidationError> { ... }

// Document public APIs
/// Validates a Plan's graph for cycles and invalid references.
///
/// # Errors
/// Returns `ValidationError::CyclicGraph` if cycles are detected.
pub fn validate_graph(plan: &Plan) -> Result<(), ValidationError> { ... }

// Prefer iterators over manual loops
let node_ids: HashSet<_> = plan.spec.graph.nodes.iter().map(|n| &n.id).collect();

// Use ? for error propagation
let plan = client.get_plan(&name)?;
```

### JSON Schema

```json
{
  "$id": "https://planspec.io/schemas/v1alpha1/goal.schema.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Goal",
  "description": "A high-level objective with acceptance criteria",
  "type": "object",
  "required": ["apiVersion", "kind", "metadata", "spec"],
  "properties": {
    ...
  }
}
```

Guidelines:
- Use `$id` with full URI
- Include `title` and `description`
- Use `$ref` for shared definitions
- Keep `additionalProperties: false` by default

### YAML Examples

```yaml
# Good: clear, documented, realistic
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: enable-google-sso           # DNS-subdomain format
  namespace: team-platform
  labels:
    team: platform
    priority: high
spec:
  description: "Enable Google SSO for the application"
  # ... realistic fields
```

### Markdown

- Use ATX-style headers (`#`)
- One sentence per line (for better diffs)
- Use fenced code blocks with language hints
- Include table of contents for long documents

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_plan_rejects_cycles() {
        let plan = Plan {
            spec: PlanSpec {
                graph: Graph {
                    nodes: vec![
                        Node { id: "a".into(), .. },
                        Node { id: "b".into(), .. },
                    ],
                    edges: vec![
                        Edge { from: "a".into(), to: "b".into() },
                        Edge { from: "b".into(), to: "a".into() }, // Cycle!
                    ],
                },
                ..Default::default()
            },
            ..Default::default()
        };

        let result = validate_plan_graph(&plan);
        assert!(matches!(result, Err(ValidationError::CyclicGraph(_))));
    }
}
```

### Schema Tests

```javascript
// schemas/test/goal.test.js
const Ajv = require('ajv');
const goalSchema = require('../v1alpha1/goal.schema.json');

describe('Goal Schema', () => {
  it('validates a correct goal', () => {
    const goal = {
      apiVersion: 'planspec.io/v1alpha1',
      kind: 'Goal',
      metadata: { name: 'test-goal', namespace: 'default' },
      spec: { description: 'A test goal' }
    };
    expect(validate(goalSchema, goal)).toBe(true);
  });

  it('rejects goal without description', () => {
    const goal = {
      apiVersion: 'planspec.io/v1alpha1',
      kind: 'Goal',
      metadata: { name: 'test-goal', namespace: 'default' },
      spec: {}
    };
    expect(validate(goalSchema, goal)).toBe(false);
  });
});
```

### Integration Tests

```bash
# Test CLI against example files
for f in docs/examples/*.yaml; do
  planspec validate -f "$f" || exit 1
done
```

---

## Release Process

### Versioning

PlanSpec follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking API changes
- **MINOR**: New features, backwards compatible
- **PATCH**: Bug fixes, backwards compatible

API version (`v1alpha1`) is separate from release version.

### Release Checklist

1. **Update CHANGELOG.md**
2. **Bump versions**:
   - `cli/Cargo.toml`
   - Schema `$id` URLs (if API version changes)
3. **Create release PR**
4. **Tag after merge**: `git tag v0.1.0`
5. **Publish**:
   - GitHub Release
   - crates.io (CLI)
   - npm (schemas, optional)

### Pre-release Versions

For testing:
- `v0.1.0-alpha.1`
- `v0.1.0-beta.1`
- `v0.1.0-rc.1`

---

## Questions?

- **Issues**: [GitHub Issues](https://github.com/exponential-build/planspec/issues)
- **Discussions**: [GitHub Discussions](https://github.com/exponential-build/planspec/discussions)

Thank you for contributing!
