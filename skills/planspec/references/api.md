# PlanSpec API Reference

Complete REST API reference for PlanSpec v1alpha1.

## Base URL

```
http://localhost:9090/apis/planspec.io/v1alpha1
```

All resources are namespaced. The URL pattern is:

```
/apis/planspec.io/v1alpha1/namespaces/{namespace}/{resource}
```

Resource plurals: `goals`, `plans`, `capabilities`, `bindings`, `executions`, `gates`

## Endpoints

### CRUD (all resource types)

| Operation | Method | Path | Notes |
|-----------|--------|------|-------|
| **List** | GET | `/namespaces/{ns}/{resource}` | Returns `{kind}List` wrapper |
| **List all** | GET | `/{resource}` | Cross-namespace listing |
| **Get** | GET | `/namespaces/{ns}/{resource}/{name}` | 404 if not found |
| **Create** | POST | `/namespaces/{ns}/{resource}` | 409 if already exists |
| **Replace** | PUT | `/namespaces/{ns}/{resource}/{name}` | Full replacement; supports optimistic concurrency |
| **Delete** | DELETE | `/namespaces/{ns}/{resource}/{name}` | 404 if not found |
| **Update status** | PUT | `/namespaces/{ns}/{resource}/{name}/status` | Replaces only `status`, preserves `spec` |

### Special Endpoints

| Operation | Method | Path | Notes |
|-----------|--------|------|-------|
| **Apply** | POST | `/namespaces/{ns}/apply` | Bulk create/update; skips resourceVersion checks |
| **Validate** | POST | `/validate` | Schema validation only, no persistence |
| **Watch** | GET | `/namespaces/{ns}/{resource}/watch` | SSE stream of change events |
| **Plan graph** | GET | `/namespaces/{ns}/plans/{name}/graph` | Returns `spec.graph` directly |
| **List namespaces** | GET | `/namespaces` | Returns `NamespaceList` |
| **Get namespace** | GET | `/namespaces/{name}` | Single namespace metadata |
| **Create namespace** | POST | `/namespaces` | Explicit creation (also auto-created on resource create) |
| **Delete namespace** | DELETE | `/namespaces/{name}` | Deletes namespace AND all resources in it |
| **Health** | GET | `/healthz` | Returns `"ok"` |

## Request/Response Format

All requests and responses use `Content-Type: application/json`.

### List Response

```json
{
  "apiVersion": "planspec.io/v1alpha1",
  "kind": "GoalList",
  "items": [ ... ]
}
```

### Error Response

```json
{
  "kind": "Status",
  "status": "Failure",
  "message": "Goal 'my-goal' not found in namespace 'default'",
  "reason": "NotFound",
  "code": 404
}
```

Common error codes: 400 (validation), 404 (not found), 409 (conflict/already exists), 500 (internal).

## Optimistic Concurrency

Include `metadata.resourceVersion` in PUT requests. The server returns 409 Conflict if the stored version doesn't match:

```bash
# Get current version
CURRENT=$(curl -s http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/goals/my-goal)
RV=$(echo "$CURRENT" | jq -r '.metadata.resourceVersion')

# Update with version check
curl -X PUT http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/goals/my-goal \
  -H 'Content-Type: application/json' \
  -d "$(echo "$CURRENT" | jq '.spec.description = "Updated"')"
```

If `resourceVersion` is omitted from the PUT body, the server does **not** enforce concurrency — it overwrites unconditionally.

## Status Subresource

The `/status` endpoint replaces **only** the `status` field while preserving the existing `spec`, `metadata`, and other fields. This is the correct way to update resource phase and progress.

```bash
curl -X PUT http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/executions/my-run/status \
  -H 'Content-Type: application/json' \
  -d '{
    "status": {
      "phase": "Running",
      "nodeStatuses": {
        "task-1": {"phase": "Succeeded", "message": "Done", "completionTime": "2026-01-01T00:00:00Z"},
        "task-2": {"phase": "Running", "startTime": "2026-01-01T00:01:00Z"}
      }
    }
  }'
```

⚠️ **Status PUT replaces the entire status object.** Always include ALL nodeStatuses, not just changed ones. Omitted fields are removed.

## Apply Endpoint

Bulk create-or-update. Accepts an array of resources:

```bash
curl -X POST http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/apply \
  -H 'Content-Type: application/json' \
  -d '[
    {"apiVersion": "planspec.io/v1alpha1", "kind": "Goal", "metadata": {"name": "g1"}, "spec": {"description": "..."}},
    {"apiVersion": "planspec.io/v1alpha1", "kind": "Plan", "metadata": {"name": "p1"}, "spec": {"goalRef": {"name": "g1"}, "graph": {"nodes": [], "edges": []}}}
  ]'
```

⚠️ Apply **skips resourceVersion checks** — it always overwrites. Use for idempotent setup, not concurrent updates.

## Watch Endpoint (SSE)

Server-Sent Events stream of resource changes:

```bash
curl -N http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/gates/watch
```

Events are filtered by namespace and resource kind. Each SSE `data` field contains a JSON event object with `type` (ADDED/MODIFIED/DELETED) and `object` (the full resource).

## Resource Schemas

### Goal

**spec fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `description` | string | Yes | What to achieve |
| `constraints` | string | No | Constraints on solutions |
| `acceptanceCriteria` | array | No | `[{id, description}]` |
| `planSelector` | object | No | `{matchLabels, matchExpressions}` — selects plans |
| `timeout` | string | No | Duration string |
| `priority` | string | No | Priority level |
| `context` | array | No | Context attachments |

**status fields:** `phase` (Pending/Planning/Ready/Executing/Succeeded/Failed/Cancelled), `activePlanRef`, `conditions`, `observedGeneration`

### Plan

**spec fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `goalRef` | ObjectReference | No | Links to a Goal |
| `series` | string | No | Plan series name |
| `version` | string | No | Version within series |
| `supersedes` | array | No | Previous plan references |
| `description` | string | Yes | Plan description |
| `graph` | object | Yes | `{nodes: [], edges: []}` — the DAG |
| `graphDigest` | string | No | Hash of graph structure |
| `context` | array | No | Context attachments |

**status fields:** `phase` (Ready/Invalid), `nodeCount`, `conditions`, `observedGeneration`

**Graph nodes** have `id` (DNS label), `kind` (task/gate/group/external), and kind-specific fields. See SKILL.md for node kind details.

**Graph edges** have `from`, `to`, and `type` (hard/soft).

### Execution

**spec fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `goalRef` | ObjectReference | No | Links to a Goal |
| `planRef` | ObjectReference | Yes | Links to a Plan (immutable after create) |
| `bindingRef` | ObjectReference | No | Links to a Binding |
| `nodeBindings` | object | No | Per-node binding overrides |
| `runtimeRef` | ObjectReference | No | Runtime reference |
| `parameters` | object | No | Runtime parameters |
| `context` | array | No | Context attachments |

**status fields:**

| Field | Type | Description |
|-------|------|-------------|
| `phase` | string | Pending/Running/Blocked/Succeeded/Failed/Cancelled |
| `runId` | string | Unique run identifier |
| `reason` | string | Machine-readable reason |
| `message` | string | Human-readable message |
| `queuedTime` | datetime | When queued |
| `startTime` | datetime | When started |
| `completionTime` | datetime | When completed |
| `lastPhaseTransitionTime` | datetime | Last phase change |
| `specHash` | string | Hash of spec at start |
| `nodeStatuses` | map | `{nodeId: NodeStatus}` |
| `artifacts` | array | Execution-level artifacts |
| `conditions` | array | Standard conditions |
| `observedGeneration` | int | Last reconciled generation |
| `extensions` | object | Free-form extensions |

**NodeStatus fields:** `phase` (Pending/Running/Blocked/Succeeded/Failed/Skipped/Cancelled), `startTime`, `completionTime`, `attempt`, `outputs`, `message`, `extensions`

**Artifact fields:** `name`, `type`, `path`, `url` (⚠️ not `uri`), `checksum`, `size`, `mediaType`, `metadata`

### Gate

**spec fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `gateType` | string | Yes | `approval`, `review`, or `sign-off` |
| `targetRef` | object | Yes | `{apiVersion, kind, name, namespace, uid, nodeId}` — what this gate guards |
| `description` | string | No | What to review/approve |
| `reviewers` | array | No | `["user:name", ...]` |
| `requiredApprovers` | int | No | Minimum approvals needed |
| `context` | array | No | Context attachments |

**status fields:**

| Field | Type | Description |
|-------|------|-------------|
| `phase` | string | Pending/Waiting/Approved/Rejected |
| `conditions` | array | Standard conditions |
| `reviewHistory` | array | `[{reviewer, action, timestamp, comment, targetGeneration}]` |
| `resolution` | object | `{outcome, actors, timestamp, comment}` — required for terminal phase |
| `observedGeneration` | int | Last reconciled generation |
| `decidedGeneration` | int | Gate generation when decided — required for terminal phase |

**Schema invariants:**
- Terminal phase (Approved/Rejected) **requires** both `resolution` and `decidedGeneration`
- `resolution.outcome` must match phase: `"approved"` ↔ `"Approved"`, `"rejected"` ↔ `"Rejected"`

### Capability

**spec fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `displayName` | string | No | Human-friendly name |
| `description` | string | Yes | What this capability provides |
| `category` | string | No | Grouping category |
| `inputs` | array | No | `[{name, type, description, required, default, format, enum, itemType, objectProperties}]` |
| `outputs` | array | No | Same schema as inputs |
| `requirements` | array | No | `[{name, namespace, optional}]` — other capabilities required |

**status fields:** `phase` (Available/Unavailable/Deprecated), `conditions`, `observedGeneration`, `replacement`, `deprecationMessage`

### Binding

**spec fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `strategy` | string | No | Binding strategy |
| `rules` | array | Yes | `[{priority, selector, target}]` |

**Rule selector:** `{capabilityRef, planRef, nodeId, matchLabels}`
**Rule target:** `{provider, config}`

**status fields:** `phase` (Unresolved/PartiallyBound/Ready), `totalCount`, `boundCount`, `unboundCapabilities`, `conditions`, `observedGeneration`

## Context Attachments

Reusable across Goal, Plan, and Execution specs:

```yaml
context:
  - name: design-notes
    content: "Inline markdown content"
    format: markdown
  - name: references
    format: uri-list
    uris:
      - file:///path/to/local/file
      - https://example.com/doc
```

## Namespace Auto-Creation

Creating any resource in a non-existent namespace **automatically creates the namespace**. You don't need to create namespaces explicitly unless you want to manage them independently.

## Server-Populated Fields

On resource creation, the server automatically sets:
- `metadata.uid`, `metadata.resourceVersion`, `metadata.generation`, `metadata.creationTimestamp`
- `metadata.namespace` (from URL path, overrides body)
- `status.phase` (initial phase per resource kind, unless client provides one)
- `status.observedGeneration`
- `status.conditions` (initial condition)

For Plans specifically, the server also sets `status.nodeCount` from the graph.
