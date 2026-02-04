# PlanSpec Conventions

This document describes the naming, labeling, and structural conventions for PlanSpec resources.

## Naming Conventions

### Resource Names (`metadata.name`)

Resource names must follow DNS label format (RFC 1123):

- **Lowercase only**: `my-goal` not `My-Goal`
- **Start with a letter**: `goal-1` not `1-goal`
- **Letters, numbers, hyphens only**: `my-goal-v1` not `my_goal_v1`
- **No trailing hyphens**: `my-goal` not `my-goal-`
- **Maximum 63 characters**

**Valid examples:**
```
my-goal
planspec-v0-ready
api-server-lite-v2
test-123
```

**Invalid examples:**
```
My-Goal          # uppercase
123-test         # starts with number
my_goal          # underscore
my-goal-         # trailing hyphen
```

### Namespace Names (`metadata.namespace`)

Same rules as resource names (DNS label format).

**Common namespaces:**
- `default` - Default namespace for resources
- `planspec` - PlanSpec's own dogfooding resources

### Plan Node IDs (`spec.graph.nodes[].id`)

Node IDs are slightly more permissive than resource names:

- **Lowercase only**
- **Start with a letter**
- **Letters, numbers, hyphens, and underscores allowed**
- **Maximum 63 characters**

**Valid examples:**
```
step-1
schema-and-conventions
v0-ready
my_task
```

## Label Conventions

### System Labels

PlanSpec system labels use the `planspec.io/` prefix:

| Label | Description | Example |
|-------|-------------|---------|
| `planspec.io/goal` | Goal this Plan satisfies | `planspec.io/goal: my-goal` |
| `planspec.io/series` | Plan series identifier | `planspec.io/series: my-plan` |
| `planspec.io/version` | Plan version within series | `planspec.io/version: "2"` |

**Example Plan with labels:**
```yaml
apiVersion: planspec.io/v1alpha1
kind: Plan
metadata:
  name: my-plan-v2
  namespace: default
  labels:
    planspec.io/goal: my-goal
    planspec.io/series: my-plan
    planspec.io/version: "2"
```

### User Labels

User-defined labels should use a custom prefix to avoid conflicts:

```yaml
labels:
  team.example.com/owner: platform
  team.example.com/priority: high
```

## Annotation Conventions

### System Annotations

System annotations use the `planspec.io/` prefix:

| Annotation | Description |
|------------|-------------|
| `planspec.io/description` | Extended description |
| `planspec.io/created-by` | Tool or user that created the resource |

### User Annotations

User annotations should use a custom prefix:

```yaml
annotations:
  team.example.com/jira-ticket: PROJ-123
  team.example.com/review-date: "2024-01-15"
```

## Plan Versioning

Plans use `spec.series` and `spec.version` for versioning:

```yaml
spec:
  series: my-plan        # Stable identifier for plan family
  version: "2"           # Version within the series
  supersedes:            # Optional: plans this version replaces
    - name: my-plan-v1
```

**Resolution rules:**
1. Plans are grouped by `spec.series`
2. Within each series, highest `spec.version` is selected
3. Plans listed in `spec.supersedes` are excluded

## Goal-Plan Relationships

Plans link to Goals via `spec.goalRef`:

```yaml
# Goal
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: implement-feature-x
  namespace: default
spec:
  description: "Implement feature X"
  planSelector:              # Optional: filter plans by labels
    matchLabels:
      planspec.io/series: feature-x-plan

---
# Plan
apiVersion: planspec.io/v1alpha1
kind: Plan
metadata:
  name: feature-x-plan-v1
  namespace: default
  labels:
    planspec.io/goal: implement-feature-x
    planspec.io/series: feature-x-plan
    planspec.io/version: "1"
spec:
  goalRef:
    name: implement-feature-x
  series: feature-x-plan
  version: "1"
```

## Validation

These conventions are enforced programmatically:

- **Schema validation**: JSON schemas validate structure
- **Name validation**: `metadata.name` and `metadata.namespace` checked against DNS label format
- **Node ID validation**: `spec.graph.nodes[].id` checked for valid format
- **Graph validation**: Cycles detected, edge references verified

Validation errors include clear messages:

```
invalid metadata.name: 'My-Goal' - must be lowercase, start with letter, contain only letters, numbers, and hyphens
```

## File Organization

Recommended directory structure for PlanSpec resources:

```
planspec/
├── goals/           # Goal definitions
├── plans/           # Plan definitions
├── capabilities/    # Capability definitions
├── bindings/        # Binding rules
└── executions/      # Execution records (often ephemeral)
```

## API Version

All v1alpha1 resources use:

```yaml
apiVersion: planspec.io/v1alpha1
```
