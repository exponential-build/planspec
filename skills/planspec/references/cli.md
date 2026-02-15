# PlanSpec CLI Reference

The PlanSpec CLI (`planspec`) provides kubectl-style management of PlanSpec resources.

**Binary:** `/home/node/.openclaw/workspace/planspec/target/debug/planspec`
**Env:** `PLANSPEC_SERVER=http://localhost:9090`

## Environment Variables

| Variable | Description | Example |
|---|---|---|
| `PLANSPEC_SERVER` | API server URL | `http://localhost:9090` |
| `PLANSPEC_NAMESPACE` | Default namespace | `exponential` |
| `EDITOR` / `VISUAL` | Editor for `planspec edit` | `vim` |

## Global Options

All subcommands accept:

| Flag | Short | Description |
|---|---|---|
| `--server <URL>` | | API server URL (overrides `PLANSPEC_SERVER`) |
| `--namespace <NS>` | `-n` | Namespace (overrides `PLANSPEC_NAMESPACE`) |
| `--output <FMT>` | `-o` | Output format: `table` (default), `json`, `yaml` |
| `--help` | `-h` | Show help |
| `--version` | `-V` | Show version |

## Commands

### apply — Apply resources from files

Apply (create or update) resources from YAML files.

```bash
# Apply a single file
planspec apply -f goal.yaml

# Apply all YAML files in a directory
planspec apply -d ./specs/

# Recursive directory apply
planspec apply -d ./specs/ -R

# Dry run — validate without applying
planspec apply -f plan.yaml --dry-run
```

**curl equivalent:**
```bash
# Create
curl -X POST http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/NS/goals \
  -H 'Content-Type: application/json' -d @goal.json

# Update (PUT to named resource)
curl -X PUT http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/NS/goals/NAME \
  -H 'Content-Type: application/json' -d @goal.json
```

| Flag | Description |
|---|---|
| `-f, --file <FILE>` | File to apply |
| `-d, --directory <DIR>` | Directory of YAML files |
| `-R, --recursive` | Recurse into subdirectories (with `-d`) |
| `--dry-run` | Validate only, don't apply |

---

### get — List or retrieve resources

List all resources of a type, or get a specific one by name.

```bash
# List all goals in current namespace
planspec get goals

# Get a specific plan
planspec get plans my-plan

# List across all namespaces
planspec get executions -A

# YAML output
planspec get gates -o yaml

# Filter by label selector
planspec get plans -s "planspec.io/goal=my-goal"

# Filter plans by series
planspec get plans --series my-series

# List everything
planspec get all
```

**curl equivalent:**
```bash
# List
curl http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/NS/goals

# Get specific
curl http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/NS/goals/NAME

# All namespaces
curl http://localhost:9090/apis/planspec.io/v1alpha1/goals
```

| Argument | Description |
|---|---|
| `<RESOURCE>` | Resource type: `goals`, `plans`, `capabilities`, `bindings`, `executions`, `gates`, `all` |
| `[NAME]` | Optional — specific resource name |

| Flag | Description |
|---|---|
| `-A, --all-namespaces` | List from all namespaces |
| `-s, --selector <SEL>` | Label selector filter |
| `--series <SERIES>` | Filter plans by series label |

---

### describe — Show detailed info

Show a resource in human-readable detail (richer than `get`).

```bash
planspec describe plan my-plan
planspec describe execution my-run -n exponential
```

**curl equivalent:**
```bash
curl http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/NS/plans/NAME
# (same as get, but `describe` formats it for readability)
```

---

### delete — Delete resources

Delete by type+name or by file.

```bash
# Delete by type and name
planspec delete goal old-goal

# Delete all resources defined in a file
planspec delete -f resources.yaml

# Delete from a directory
planspec delete -d ./old-specs/ -R
```

**curl equivalent:**
```bash
curl -X DELETE http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/NS/goals/NAME
```

---

### create — Create a resource

Currently supports namespace creation only.

```bash
planspec create namespace my-namespace
```

**curl equivalent:**
```bash
curl -X POST http://localhost:9090/apis/planspec.io/v1alpha1/namespaces \
  -H 'Content-Type: application/json' \
  -d '{"name": "my-namespace"}'
```

---

### validate — Offline schema validation

Validate YAML files against PlanSpec JSON schemas without contacting the server.

```bash
planspec validate -f plan.yaml
planspec validate -d ./specs/ -R
```

No curl equivalent — this is a client-side operation.

---

### watch — Watch for changes

Stream real-time changes to resources (Server-Sent Events).

```bash
planspec watch executions
planspec watch gates -n exponential
planspec watch plans -s "planspec.io/goal=my-goal"
```

**curl equivalent:**
```bash
curl -N http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/NS/executions?watch=true
```

---

### diff — Compare file to server state

Show what would change if you applied a file.

```bash
planspec diff -f updated-plan.yaml
```

No direct curl equivalent — requires client-side diffing.

---

### graph — Visualize a plan's DAG

Render a plan's dependency graph.

```bash
# Text (ASCII) output
planspec graph my-plan

# Graphviz DOT format
planspec graph my-plan --format dot

# Mermaid format (for docs/markdown)
planspec graph my-plan --format mermaid
```

| Flag | Description |
|---|---|
| `--format <FMT>` | `text` (default), `dot`, `mermaid` |

No direct curl equivalent — client-side rendering from plan data.

---

### edit — Edit a resource interactively

Opens the resource in your editor. On save, applies the changes.

```bash
planspec edit plan my-plan
planspec edit execution my-run --editor nano
```

Uses `$EDITOR`, `$VISUAL`, or falls back to `vi`.

**curl equivalent:** GET → edit locally → PUT back.

---

### completions — Generate shell completions

```bash
planspec completions bash > /etc/bash_completion.d/planspec
planspec completions zsh > ~/.zfunc/_planspec
planspec completions fish > ~/.config/fish/completions/planspec.fish
```

---

## Common Patterns

### Apply and verify
```bash
planspec apply -f my-plan.yaml
planspec get plans my-plan -o yaml
```

### Check execution progress
```bash
planspec get executions -A -o yaml
# or for a specific one:
planspec describe execution my-run -n exponential
```

### Update execution node status (via API)
```bash
curl -X PUT http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/NS/executions/NAME/status \
  -H 'Content-Type: application/json' \
  -d '{
    "nodeStatuses": {
      "task-id": {
        "phase": "Succeeded",
        "message": "Task completed",
        "completedAt": "2026-02-15T07:00:00Z"
      }
    }
  }'
```

### Approve a gate (via API)
```bash
curl -X PUT http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/NS/gates/GATE-NAME/status \
  -H 'Content-Type: application/json' \
  -d '{
    "phase": "Approved",
    "resolution": {
      "outcome": "approved",
      "actors": ["user:reviewer"],
      "timestamp": "2026-02-15T07:00:00Z"
    }
  }'
```

### Watch gates for approvals
```bash
planspec watch gates -n exponential
```

## Resource Types

| CLI name | Kind | Shorthand |
|---|---|---|
| `goals` | Goal | — |
| `plans` | Plan | — |
| `capabilities` | Capability | — |
| `bindings` | Binding | — |
| `executions` | Execution | — |
| `gates` | Gate | — |
| `all` | (all types) | — |
