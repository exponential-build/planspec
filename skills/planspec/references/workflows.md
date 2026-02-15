# PlanSpec Workflow Reference

Common agent workflows for operating PlanSpec. Each workflow includes step-by-step instructions with API calls.

## 1. Create and Execute a Plan

The full lifecycle from idea to tracked execution.

### Step 1: Create a Goal

```bash
curl -X POST http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/goals \
  -H 'Content-Type: application/json' \
  -d '{
    "apiVersion": "planspec.io/v1alpha1",
    "kind": "Goal",
    "metadata": {"name": "my-feature", "namespace": "default"},
    "spec": {
      "description": "Ship the new feature with proper review gates",
      "acceptanceCriteria": [
        {"name": "feature-deployed", "type": "endpoint_responds", "url": "http://localhost:8080/health", "expectedStatus": 200}
      ]
    }
  }'
```

### Step 2: Create Gate Resources

**Create gates BEFORE the plan** — the plan references them via `gateRef`.

```bash
# Plan approval gate (every plan should have one)
curl -X POST http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/gates \
  -H 'Content-Type: application/json' \
  -d '{
    "apiVersion": "planspec.io/v1alpha1",
    "kind": "Gate",
    "metadata": {"name": "my-feature-plan-approval", "namespace": "default"},
    "spec": {
      "gateType": "approval",
      "reviewers": ["user:alice"],
      "description": "Approve the plan before work begins",
      "targetRef": {"kind": "Plan", "name": "my-feature-v1", "nodeId": "plan-approval"}
    }
  }'

# PR review gate
curl -X POST http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/gates \
  -H 'Content-Type: application/json' \
  -d '{
    "apiVersion": "planspec.io/v1alpha1",
    "kind": "Gate",
    "metadata": {"name": "my-feature-pr-review", "namespace": "default"},
    "spec": {
      "gateType": "review",
      "reviewers": ["user:alice"],
      "description": "Review the implementation PR",
      "targetRef": {"kind": "Plan", "name": "my-feature-v1", "nodeId": "pr-review"}
    }
  }'
```

### Step 3: Create the Plan

```bash
curl -X POST http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/plans \
  -H 'Content-Type: application/json' \
  -d '{
    "apiVersion": "planspec.io/v1alpha1",
    "kind": "Plan",
    "metadata": {"name": "my-feature-v1", "namespace": "default", "labels": {"planspec.io/goal": "my-feature", "planspec.io/series": "my-feature", "planspec.io/version": "1"}},
    "spec": {
      "description": "Implementation plan for the feature",
      "goalRef": {"name": "my-feature"},
      "series": "my-feature",
      "version": "1",
      "graph": {
        "nodes": [
          {"id": "plan-approval", "kind": "Gate", "gateRef": {"name": "my-feature-plan-approval"}, "description": "Approve plan"},
          {"id": "task-implement", "kind": "Task", "description": "Implement the feature", "estimatedEffort": "medium", "acceptanceCriteria": [{"name": "tests-pass", "type": "command_succeeds", "command": "npm", "args": ["test"]}]},
          {"id": "pr-review", "kind": "Gate", "gateRef": {"name": "my-feature-pr-review"}, "description": "Review PR"}
        ],
        "edges": [
          {"from": "plan-approval", "to": "task-implement", "type": "hard"},
          {"from": "task-implement", "to": "pr-review", "type": "hard"}
        ]
      }
    }
  }'
```

### Step 4: Create an Execution

```bash
curl -X POST http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/executions \
  -H 'Content-Type: application/json' \
  -d '{
    "apiVersion": "planspec.io/v1alpha1",
    "kind": "Execution",
    "metadata": {"name": "my-feature-run-1", "namespace": "default", "labels": {"planspec.io/goal": "my-feature"}},
    "spec": {
      "goalRef": {"name": "my-feature", "namespace": "default"},
      "planRef": {"name": "my-feature-v1", "namespace": "default"}
    }
  }'
```

### Step 5: Update Task Progress

As you work on tasks, update execution `nodeStatuses`:

```bash
# Mark task as Running
curl -X PUT http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/executions/my-feature-run-1/status \
  -H 'Content-Type: application/json' \
  -d '{
    "status": {
      "phase": "Running",
      "nodeStatuses": {
        "task-implement": {
          "phase": "Running",
          "startedAt": "2026-02-15T08:00:00Z",
          "message": "Working on implementation"
        }
      }
    }
  }'

# Mark task as Succeeded (with artifacts)
curl -X PUT http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/executions/my-feature-run-1/status \
  -H 'Content-Type: application/json' \
  -d '{
    "status": {
      "phase": "Running",
      "nodeStatuses": {
        "task-implement": {
          "phase": "Succeeded",
          "startedAt": "2026-02-15T08:00:00Z",
          "completedAt": "2026-02-15T10:00:00Z",
          "message": "Implementation complete, PR submitted",
          "artifacts": [{"name": "pr", "type": "url", "url": "https://github.com/org/repo/pull/42"}]
        }
      }
    }
  }'
```

**Important:** Use `url` (not `uri`) for artifact URLs. This is a known gotcha.

## 2. Gate Approval Flow

How gates move through their lifecycle. **Agents NEVER auto-approve gates.**

### Check for Pending/Waiting Gates

```bash
# List all gates across namespaces
planspec get gates -A -o yaml

# Or via API
curl http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/gates
```

### When a Gate is Approved (by a human)

After a human approves a gate, the agent must sync the execution:

1. **Check** the gate's `status.phase` — confirm it's `Approved`
2. **Update** the execution's `nodeStatuses` for that gate node to `Succeeded`

```bash
# Gate was approved — now update execution nodeStatus to match
curl -X PUT http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/executions/my-feature-run-1/status \
  -H 'Content-Type: application/json' \
  -d '{
    "status": {
      "phase": "Running",
      "nodeStatuses": {
        "plan-approval": {
          "phase": "Succeeded",
          "message": "Gate approved",
          "completionTime": "2026-02-15T08:30:00Z"
        }
      }
    }
  }'
```

**Critical:** Both the Gate resource AND the execution nodeStatus must reflect the approval. If you only update one, the system gets out of sync.

### Notify Humans About Blocking Gates

When a gate reaches a state where it needs human attention, notify them:
- Send a message with gate name, description, what it's blocking
- Never nag — check once, notify once, wait for action

## 3. Plan Versioning

Plans use `series` + `version` + `supersedes` for lineage.

### Creating a New Version

```yaml
apiVersion: planspec.io/v1alpha1
kind: Plan
metadata:
  name: my-feature-v2
  labels:
    planspec.io/series: my-feature
    planspec.io/version: "2"
spec:
  series: my-feature
  version: "2"
  supersedes:
    - name: my-feature-v1
  # ... updated graph
```

### Key Rules

- `series` stays the same across versions
- `version` increments (string, not int)
- `supersedes` lists the plan(s) this version replaces
- Labels `planspec.io/series` and `planspec.io/version` enable selector-based queries
- Old executions on v1 continue running — they don't auto-migrate

## 4. Parallel Agent Coordination

Multiple agents can work on different tasks within the same execution.

### Ownership Pattern

Each agent should own specific node IDs and only update those:

```bash
# Agent A updates its task
curl -X PUT .../executions/run-1/status \
  -d '{"status": {"nodeStatuses": {"task-frontend": {"phase": "Running"}}}}'

# Agent B updates its task (independently)
curl -X PUT .../executions/run-1/status \
  -d '{"status": {"nodeStatuses": {"task-backend": {"phase": "Running"}}}}'
```

### Conflict Avoidance

- **Check before updating:** GET the execution, read current nodeStatuses
- **Only update your nodes:** Don't overwrite other agents' statuses
- **Use `resourceVersion`** for optimistic concurrency if supported
- **Gate nodes are shared:** Only the gate approval system should update gate nodeStatuses

### Sequencing Dependencies

Use the plan's `edges` to determine task ordering:
- A task is **unblocked** when all upstream dependencies have `phase: Succeeded`
- A task behind a gate is unblocked only when the gate's `phase` is `Approved` (on the Gate resource) AND the gate's execution nodeStatus is `Succeeded`

## 5. Monitoring and Observability

### Check Execution Progress

```bash
# All executions across namespaces
planspec get executions -A -o yaml

# Specific execution
planspec get execution my-feature-run-1 -n default -o yaml

# Via API
curl http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/executions/my-feature-run-1
```

### Check Gate Queue

Find all gates needing attention:

```bash
# All gates
planspec get gates -A -o yaml

# Filter for non-approved (in your processing logic)
# Look for gates where status.phase is empty, "Pending", or "Waiting"
```

### Watch for Real-Time Updates

```bash
# Watch executions for changes (SSE stream)
curl -N http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/executions?watch=true

# Watch gates
curl -N http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/gates?watch=true
```

### Finding Unblocked Tasks

Algorithm for the work loop:

1. GET all executions (`-A` for all namespaces)
2. For each execution with `phase: Running`:
   a. GET the referenced plan (from `spec.planRef`)
   b. Read the plan's `graph.edges` to build a dependency map
   c. For each node in `nodeStatuses`:
      - Skip if `phase` is `Succeeded`, `Running`, or `Failed`
      - Check all upstream nodes (via edges): all must be `Succeeded`
      - For gate nodes: check the Gate resource's `status.phase` is `Approved`
      - If all deps satisfied → task is **unblocked**
3. Pick an unblocked task, do the work, update nodeStatus

## 6. Working with Context

Plans and executions support context attachments for additional information:

```yaml
spec:
  context:
    - name: design-notes
      format: markdown
      content: |
        Key design decisions...
    - name: api-docs
      format: uri-list
      uris:
        - https://docs.example.com/api
        - file:///local/path/to/spec.md
```

Context is informational — it helps agents understand what to do but doesn't affect execution flow.
