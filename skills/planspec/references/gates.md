# PlanSpec Gate Reference

Gates are the human oversight checkpoints in PlanSpec. They exist so that agents never proceed past decision points without explicit human approval.

## Gate Types

| Type | Purpose | Use Case |
|---|---|---|
| `approval` | Simple yes/no checkpoint | Plan approval, milestone sign-off |
| `review` | Code/design review (may link to PR) | PR review, architecture review |
| `sign-off` | Formal compliance sign-off | Security review, legal, compliance |

## Gate Lifecycle

```
(created) → Pending → Waiting → Approved
                              → Rejected
```

1. **Pending** — Gate resource exists but no execution has reached it yet
2. **Waiting** — Execution has reached the gate node; human action required
3. **Approved** — Reviewer(s) approved; downstream tasks can proceed
4. **Rejected** — Reviewer(s) rejected; execution blocks

### How Gates Connect to Plans

A gate is linked to a plan node via `spec.targetRef`:

```yaml
apiVersion: planspec.io/v1alpha1
kind: Gate
metadata:
  name: my-plan-approval
  namespace: exponential
spec:
  gateType: approval
  description: Approve the plan before work begins
  reviewers:
    - user:alice
  targetRef:
    kind: Plan
    name: my-plan-v1
    nodeId: plan-approval    # matches the node id in the plan graph
```

In the plan graph, the gate node references the Gate resource:

```yaml
nodes:
  - id: plan-approval
    kind: Gate
    description: Team lead approves the plan
    gateRef:
      name: my-plan-approval
```

### Gate Status vs Execution NodeStatus

These are **two separate things** that must stay in sync:

| Resource | Field | What it tracks |
|---|---|---|
| Gate | `status.phase` | The gate's own approval status |
| Execution | `status.nodeStatuses[nodeId].phase` | The node's progress in the execution |

When a gate is approved, you must update **both**:
1. The Gate resource's status (phase: Approved)
2. The Execution's nodeStatus for that gate node (phase: Succeeded)

If you only update one, the system gets out of sync.

## Creating Gates

```bash
# Via CLI
planspec apply -f gate.yaml

# Via API
curl -X POST http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/exponential/gates \
  -H 'Content-Type: application/json' \
  -d '{
    "apiVersion": "planspec.io/v1alpha1",
    "kind": "Gate",
    "metadata": {"name": "my-gate", "namespace": "exponential"},
    "spec": {
      "gateType": "approval",
      "description": "Approve before proceeding",
      "reviewers": ["user:alice"],
      "targetRef": {
        "kind": "Plan",
        "name": "my-plan-v1",
        "nodeId": "plan-approval"
      }
    }
  }'
```

## Approving Gates (Status Update)

Gates are approved via the status subresource:

```bash
curl -X PUT http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/exponential/gates/my-gate/status \
  -H 'Content-Type: application/json' \
  -d '{
    "status": {
      "phase": "Approved",
      "decidedGeneration": 1,
      "resolution": {
        "outcome": "approved",
        "actors": ["user:alice"],
        "timestamp": "2026-02-15T07:00:00Z"
      },
      "reviewHistory": [
        {
          "action": "approve",
          "reviewer": "user:alice",
          "timestamp": "2026-02-15T07:00:00Z"
        }
      ]
    }
  }'
```

### Key Fields

| Field | Description |
|---|---|
| `status.phase` | `Pending`, `Waiting`, `Approved`, `Rejected` |
| `status.decidedGeneration` | Must match `metadata.generation` to be valid |
| `status.resolution.outcome` | `approved` or `rejected` |
| `status.resolution.actors` | Who made the decision |
| `status.reviewHistory[]` | Audit trail of all review actions |

## Agent Gate Rules (DOs)

- **DO** create Gate resources when creating plans with gate nodes
- **DO** set gate `targetRef` correctly (kind, name, nodeId)
- **DO** update execution nodeStatus when completing tasks
- **DO** notify humans when gates reach Waiting state
- **DO** check gate phase before proceeding past gate nodes
- **DO** include meaningful `reviewHistory` entries
- **DO** set `decidedGeneration` to match the gate's `spec` generation
- **DO** always update BOTH gate status AND execution nodeStatus on approval

## Agent Gate Rules (DON'Ts)

- **NEVER** auto-approve gates — gates exist for human oversight
- **NEVER** skip gate checks even if the answer seems "obvious"
- **NEVER** modify gate spec after creation (create a new version instead)
- **NEVER** set gate phase directly — always use the `/status` subresource
- **NEVER** approve a gate without checking `decidedGeneration`
- **NEVER** forget to update BOTH gate resource AND execution nodeStatus
- **NEVER** proceed past a gate node that isn't Approved

## Common Gate Patterns

### Plan-Approval Gate (first gate in every plan)

Every plan should start with an approval gate. No work begins until the plan is approved.

```yaml
# In plan graph
edges:
  - from: plan-approval
    to: first-task
    type: hard

nodes:
  - id: plan-approval
    kind: Gate
    gateRef:
      name: my-plan-approval
```

### PR-Review Gate

Links a gate to a GitHub PR for code review:

```yaml
spec:
  gateType: review
  description: Review the implementation PR
  reviewers:
    - user:alice
  targetRef:
    kind: Plan
    name: my-plan-v1
    nodeId: pr-review
```

The PR URL is typically stored as an artifact on the preceding task's execution nodeStatus.

### Multi-Approver Gates

For gates requiring multiple reviewers:

```yaml
spec:
  gateType: sign-off
  requiredApprovers: 2
  reviewers:
    - user:alice
    - user:alice
    - user:bob
```

Gate moves to Approved only when `requiredApprovers` reviewers have approved.

## Checking Gate Status

```bash
# List all gates
planspec get gates -A -o yaml

# Check a specific gate
planspec get gates my-gate -n exponential -o yaml

# Watch for gate changes
planspec watch gates -n exponential
```

## Troubleshooting

### Gate approved but execution still blocked
The execution nodeStatus wasn't updated. Update it:
```bash
# Get current execution status, update the gate node, PUT back
curl -s http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/NS/executions/RUN/status | \
  jq '.status.nodeStatuses["gate-node"].phase = "Succeeded"' | \
  curl -X PUT http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/NS/executions/RUN/status \
    -H 'Content-Type: application/json' -d @-
```

### Gate shows empty status
The status subresource was never written. Check if anyone has approved it yet.

### Stale approval (decidedGeneration mismatch)
If the gate spec was regenerated (new generation), previous approvals become stale. The `decidedGeneration` won't match `metadata.generation`, meaning the gate needs re-approval.
