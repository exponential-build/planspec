---
name: planspec
description: Create, manage, and execute PlanSpec plans — structured work orchestration with goals, plans, executions, gates, capabilities, and bindings. Use when the user mentions planspec, plans, executions, gates, goals, approvals, DAGs, workflow orchestration, or needs structured task coordination with human oversight.
license: Apache-2.0
compatibility: Requires planspec CLI or curl. Works with any agent that has shell access.
metadata:
  author: exponential-build
  version: "1.0"
allowed-tools: Read Write Edit Bash Glob Grep TodoWrite Task WebFetch AskUserQuestion
---

# PlanSpec Skill

Declarative work orchestration for humans and AI agents. Six resource types form a DAG-based coordination system with human oversight via gates.

## Quick Start

```bash
# Set up (adjust server URL and CLI path for your environment)
export PLANSPEC_SERVER=http://localhost:9090
export PLANSPEC_NAMESPACE=default

# List all resources across namespaces
planspec get goals -A
planspec get plans -A
planspec get executions -A
planspec get gates -A
```

## Resource Model

| Kind | Purpose | Key Fields |
|------|---------|------------|
| **Goal** | What to achieve | `description`, `acceptanceCriteria[]` |
| **Plan** | How to achieve it (DAG) | `graph` (nodes + edges), `goalRef`, `series`, `version` |
| **Execution** | Running instance of a plan | `planRef` (immutable after create) |
| **Gate** | Approval/review checkpoint | `gateType`, `targetRef`, `requiredApprovers` |
| **Capability** | What an agent can do | `description`, `inputs[]`, `outputs[]` |
| **Binding** | Maps capabilities to providers | `rules[]` (selector + target) |

## Plan Graph

Plans contain a DAG of nodes connected by edges.

### Node Kinds

- **Task**: Work to do. Has `capabilityRefs`, `inputs`, `outputs`, `acceptanceCriteria`, `timeout`, `retries`
- **Gate**: Approval checkpoint. References a Gate resource via `gateRef`
- **Group**: Organizational container. `children[]` + `mode` (parallel/sequence). Presentational only — does NOT create edges
- **External**: External dependency. `externalRef` (uri/resource/webhook)

### Edges

```yaml
edges:
  - from: task-a
    to: task-b
    type: hard    # hard = blocking, soft = advisory
```

Shorthand: use `dependsOn: [node-id]` on nodes to auto-create hard edges.

## Core Workflows

### 1. Create Goal + Plan + Gate

```yaml
# goal.yaml
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: my-feature
  namespace: default
spec:
  description: "Build the feature"

# plan.yaml
apiVersion: planspec.io/v1alpha1
kind: Plan
metadata:
  name: my-feature-v1
  namespace: default
  labels:
    planspec.io/goal: my-feature
    planspec.io/series: my-feature
    planspec.io/version: "1"
spec:
  goalRef: { name: my-feature }
  series: my-feature
  version: "1"
  description: "Implementation plan"
  graph:
    nodes:
      - id: plan-approval
        kind: Gate
        gateRef: { name: my-feature-plan-approval }
      - id: implement
        kind: Task
        description: "Do the work"
      - id: review
        kind: Gate
        gateRef: { name: my-feature-review }
    edges:
      - { from: plan-approval, to: implement, type: hard }
      - { from: implement, to: review, type: hard }

# gate.yaml
apiVersion: planspec.io/v1alpha1
kind: Gate
metadata:
  name: my-feature-plan-approval
  namespace: default
spec:
  gateType: approval
  description: "Approve plan before work begins"
  reviewers: ["user:alice"]
  requiredApprovers: 1
  targetRef:
    kind: Plan
    name: my-feature-v1
    nodeId: plan-approval
```

Apply: `planspec apply -f goal.yaml -f plan.yaml -f gate.yaml`

### 2. Create and Track Execution

```bash
# Create execution (this starts it immediately — no separate "start")
planspec apply -f - <<EOF
apiVersion: planspec.io/v1alpha1
kind: Execution
metadata:
  name: my-feature-run-1
  namespace: default
spec:
  planRef: { name: my-feature-v1, namespace: default }
EOF

# Update node progress (replaces entire status — include ALL nodeStatuses)
curl -X PUT $PLANSPEC_SERVER/apis/planspec.io/v1alpha1/namespaces/default/executions/my-feature-run-1/status \
  -H 'Content-Type: application/json' \
  -d '{"status":{"phase":"Running","nodeStatuses":{"implement":{"phase":"Running","startTime":"2026-01-01T00:00:00Z"}}}}'
```

### 3. Approve a Gate

There is **no dedicated approve endpoint**. Use the status subresource:

```bash
curl -X PUT $PLANSPEC_SERVER/apis/planspec.io/v1alpha1/namespaces/default/gates/my-gate/status \
  -H 'Content-Type: application/json' \
  -d '{
    "status": {
      "phase": "Approved",
      "resolution": {"outcome":"approved","actors":["user:alice"],"timestamp":"2026-01-01T00:00:00Z"},
      "decidedGeneration": 1,
      "reviewHistory": [{"reviewer":"user:alice","action":"approve","timestamp":"2026-01-01T00:00:00Z"}]
    }
  }'
```

## API Patterns

**Base URL:** `{server}/apis/planspec.io/v1alpha1/namespaces/{ns}/{resource}`

| Operation | Method | Path |
|-----------|--------|------|
| List | GET | `/{resource}` |
| Get | GET | `/{resource}/{name}` |
| Create | POST | `/{resource}` |
| Replace | PUT | `/{resource}/{name}` |
| Delete | DELETE | `/{resource}/{name}` |
| Status | PUT | `/{resource}/{name}/status` |
| Apply | POST | `/apply` |
| Watch (SSE) | GET | `/{resource}/watch` |
| Validate | POST | `/validate` (cluster-scoped) |
| Namespaces | GET | `/namespaces` (cluster-scoped) |

## ⚠️ Critical Rules

1. **Always check ALL namespaces**: `planspec get <resource> -A`
2. **Never auto-approve gates**: Gates exist for human oversight
3. **Status PUT replaces entirely**: Include ALL nodeStatuses, not just changes
4. **Execution spec is immutable**: Can't change `planRef` after creation
5. **Create = Start**: Creating an execution starts it immediately
6. **Artifact field is `url` not `uri`**: Server uses `url` for artifact URLs
7. **Gate resolution must be complete**: Terminal phase requires `resolution` + `decidedGeneration`; outcome must match phase
8. **Graph must be acyclic**: No cycles or self-loops
9. **Group nodes are presentational**: They do NOT generate edges; use explicit edges

## Plan Versioning

Use `series` and `version` for plan evolution:

```yaml
metadata:
  name: my-feature-v2
  labels:
    planspec.io/series: my-feature
    planspec.io/version: "2"
spec:
  series: my-feature
  version: "2"
  supersedes: my-feature-v1
```

## Reference Files

Load these on demand for detailed information:

- **[references/api.md](references/api.md)** — Complete API reference: all endpoints, schemas, status subresources
- **[references/gates.md](references/gates.md)** — Gate lifecycle, types, DO/DON'T rules, troubleshooting
- **[references/workflows.md](references/workflows.md)** — End-to-end workflow patterns
- **[references/cli.md](references/cli.md)** — All CLI commands with flags and curl equivalents
- **[references/examples.md](references/examples.md)** — 16 copy-paste YAML examples for all resource types

## Helper Scripts

- **[scripts/validate.sh](scripts/validate.sh)** — Validate YAML resources structurally
- **[scripts/gate-status.sh](scripts/gate-status.sh)** — Overview table of all gates
- **[scripts/apply-plan.sh](scripts/apply-plan.sh)** — Apply goal + plan + gates in correct order

## Resource Templates

YAML templates in [assets/](assets/) for quick resource creation:
`goal.yaml`, `plan.yaml`, `execution.yaml`, `gate.yaml`, `capability.yaml`, `binding.yaml`
