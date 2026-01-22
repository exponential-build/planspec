# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build CLI
cd cli && cargo build --release

# Run tests
cd cli && cargo test

# Format and lint
cd cli && cargo fmt && cargo clippy

# Validate example specs against schemas
./target/release/planspec validate -f docs/examples/simple-goal.yaml
```

## Architecture

PlanSpec is a Kubernetes-inspired declarative specification for work orchestration. It defines contracts (not implementations) for expressing goals, plans, and executions.

### Core Primitives

All resources follow K8s conventions with `apiVersion`, `kind`, `metadata`, `spec`, `status`:

- **Goal** - High-level objective with acceptance criteria (the "what")
- **Plan** - DAG of work nodes/edges embedded in `spec.graph` (the "how")
- **Capability** - Reusable ability that agents can provide
- **Binding** - Resolution rules mapping capabilities to provider targets
- **Execution** - Immutable intent to run a Plan; `status` tracks runs

### Key Design Decisions

- **Plan is the durable artifact**: Nodes are embedded in `Plan.spec.graph`, not separate resources
- **Execution.spec is immutable**: Intent cannot change after creation; only status updates
- **Create Execution = Start**: No separate trigger action in v1alpha1
- **Binding precedence**: `planRef+nodeId` > `capabilityRef` > (future label selectors)

### CLI Structure

The Rust CLI (`cli/`) uses clap for argument parsing:
- `commands/` - Subcommands (apply, get, describe, validate, watch, diff, graph)
- `client/` - HTTP client for PlanSpec API
- `schema/` - Offline JSON Schema validation using embedded schemas
- `output/` - Table and YAML formatters

The `validate` command works offline against embedded JSON schemas. Other commands require a running PlanSpec server.

### Schema Organization

- `schemas/common/` - Shared structures (ObjectMeta, ObjectReference, Condition, Graph)
- `schemas/v1alpha1/` - Resource-specific schemas referencing common schemas
- `api/openapi/v1alpha1.yaml` - Full OpenAPI spec with K8s-style endpoints

### Important Things
- Remember to update @SPECIFICATION.md when making schema changes
- Remember to add unit tests to cover any bugs or corner cases discovered
