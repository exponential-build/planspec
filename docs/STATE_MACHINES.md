# PlanSpec State Machines

This document defines the lifecycle state machines for PlanSpec resources. **Implementations MUST follow these state machines correctly.** The server enforces initial state assignment and validates state transitions.

## Overview

All PlanSpec resources have a `status` field that tracks their lifecycle state. The `status.phase` field indicates the current state in the resource's lifecycle. The server automatically populates initial status when resources are created.

### Common Status Fields

All resources share these common status fields:

```yaml
status:
  phase: <Phase>              # Current lifecycle phase (required)
  observedGeneration: <int>   # Generation last processed by controller
  conditions:                 # Detailed status conditions (K8s-style)
    - type: <string>
      status: "True" | "False" | "Unknown"
      reason: <string>
      message: <string>
      lastTransitionTime: <RFC3339>
      observedGeneration: <int>
```

---

## Goal State Machine

Goals track the lifecycle from creation through plan selection and execution to completion.

```
                    ┌─────────┐
                    │ Pending │ ◄── Initial state (auto-set on create)
                    └────┬────┘
                         │ Plan generation started
                         ▼
                    ┌──────────┐
                    │ Planning │
                    └────┬─────┘
                         │ Plan selected/approved
                         ▼
                    ┌─────────┐
                    │  Ready  │
                    └────┬────┘
                         │ Execution started
                         ▼
                    ┌───────────┐
                    │ Executing │
                    └─────┬─────┘
           ┌──────────────┼──────────────┐
           ▼              ▼              ▼
      ┌───────────┐ ┌──────────┐ ┌───────────┐
      │ Succeeded │ │  Failed  │ │ Cancelled │
      └───────────┘ └──────────┘ └───────────┘
```

### Goal Phases

| Phase | Description | Initial Condition |
|-------|-------------|-------------------|
| `Pending` | Goal created, no plan selected | `Accepted: True` |
| `Planning` | Plan is being generated or selected | - |
| `Ready` | Plan exists and is approved, ready for execution | - |
| `Executing` | Execution is in progress | - |
| `Succeeded` | All acceptance criteria met | - |
| `Failed` | Goal could not be achieved | - |
| `Cancelled` | Goal was cancelled by user | - |

### Goal Transitions

- `Pending` → `Planning`: When plan generation starts
- `Pending` → `Ready`: When a pre-existing plan is selected
- `Planning` → `Ready`: When plan is generated and approved
- `Planning` → `Failed`: When plan generation fails
- `Ready` → `Executing`: When execution begins
- `Executing` → `Succeeded`: When all acceptance criteria pass
- `Executing` → `Failed`: When execution fails
- Any → `Cancelled`: User cancellation (terminal state)

---

## Plan State Machine

Plans have a simple validation-based state machine. Plans are immutable once created; versioning is handled via series/version fields.

```
    ┌─────────┐
    │  Ready  │ ◄── Initial state (auto-set on create if valid)
    └─────────┘
         │
         │ Validation failure (future: re-validation)
         ▼
    ┌─────────┐
    │ Invalid │
    └─────────┘
```

### Plan Phases

| Phase | Description | Initial Condition |
|-------|-------------|-------------------|
| `Ready` | Plan is valid and ready for execution | `Valid: True` |
| `Invalid` | Plan failed validation | `Valid: False` |

### Plan Status Fields

Plans have additional status fields:

```yaml
status:
  phase: Ready | Invalid
  nodeCount: <int>            # Number of nodes in the graph
  observedGeneration: <int>
  conditions:
    - type: Valid
      status: "True" | "False"
      reason: SchemaValid | ValidationFailed
      message: <string>
```

---

## Execution State Machine

Executions track the lifecycle of running a plan.

```
    ┌─────────┐
    │ Pending │ ◄── Initial state (auto-set on create)
    └────┬────┘
         │ Execution begins
         ▼
    ┌─────────┐
    │ Running │
    └────┬────┘
         │
    ┌────┴────┬───────────┐
    ▼         ▼           ▼
┌───────────┐ ┌────────┐ ┌───────────┐
│ Succeeded │ │ Failed │ │ Cancelled │
└───────────┘ └────────┘ └───────────┘
```

### Execution Phases

| Phase | Description | Initial Condition |
|-------|-------------|-------------------|
| `Pending` | Execution accepted, waiting to start | `Accepted: True` |
| `Running` | Execution is in progress | - |
| `Succeeded` | Execution completed successfully | - |
| `Failed` | Execution failed | - |
| `Cancelled` | Execution was cancelled | - |

### Execution Status Fields

```yaml
status:
  phase: Pending | Running | Succeeded | Failed | Cancelled
  runId: <string>             # Unique identifier for this run
  startTime: <RFC3339>        # When execution started
  completionTime: <RFC3339>   # When execution finished
  nodeStatuses:               # Per-node status
    <nodeId>:
      phase: <NodePhase>
      startTime: <RFC3339>
      completionTime: <RFC3339>
      attempt: <int>
      message: <string>
  artifacts:                  # Outputs from execution
    - name: <string>
      type: <string>
      uri: <string>
```

### Node Phases

Nodes within an execution have their own lifecycle:

| Phase | Description |
|-------|-------------|
| `Pending` | Node is waiting to start |
| `Running` | Node is currently executing |
| `Succeeded` | Node completed successfully |
| `Failed` | Node failed |
| `Skipped` | Node was skipped (dependency failed or condition not met) |
| `Cancelled` | Node was cancelled |

---

## Capability State Machine

Capabilities track availability for use in bindings.

```
    ┌───────────┐
    │ Available │ ◄── Initial state (auto-set on create)
    └─────┬─────┘
          │
    ┌─────┴─────┐
    ▼           ▼
┌─────────────┐ ┌────────────┐
│ Unavailable │ │ Deprecated │
└─────────────┘ └────────────┘
```

### Capability Phases

| Phase | Description | Initial Condition |
|-------|-------------|-------------------|
| `Available` | Capability is available for use | `Available: True` |
| `Unavailable` | Capability is not currently available | - |
| `Deprecated` | Capability is deprecated, avoid new usage | - |

---

## Binding State Machine

Bindings track resolution of capability references to providers.

```
    ┌────────────┐
    │ Unresolved │ ◄── Initial state (auto-set on create)
    └──────┬─────┘
           │ Resolution attempted
     ┌─────┴─────┐
     ▼           ▼
┌─────────┐ ┌────────────────┐
│  Ready  │ │ PartiallyBound │
└─────────┘ └────────────────┘
```

### Binding Phases

| Phase | Description | Initial Condition |
|-------|-------------|-------------------|
| `Unresolved` | Binding created, resolution pending | `Resolved: False` |
| `Ready` | All capabilities are bound to providers | `Resolved: True` |
| `PartiallyBound` | Some capabilities are bound, others unresolved | - |

### Binding Status Fields

```yaml
status:
  phase: Unresolved | Ready | PartiallyBound
  boundCount: <int>           # Number of resolved bindings
  unboundCapabilities:        # List of unresolved capability names
    - <string>
```

---

## Implementation Requirements

### Server Responsibilities

1. **Initial State Assignment**: The server MUST set initial `status.phase` when resources are created:
   - Goal → `Pending`
   - Plan → `Ready` (if validation passes)
   - Execution → `Pending`
   - Capability → `Available`
   - Binding → `Unresolved`

2. **Condition Initialization**: The server MUST set initial conditions with appropriate timestamps.

3. **Generation Tracking**: The server MUST set `status.observedGeneration` to match the resource's `metadata.generation`.

### Controller Responsibilities

External controllers (agents, operators) are responsible for:

1. **State Transitions**: Moving resources through their lifecycle phases
2. **Condition Updates**: Adding/updating conditions to reflect detailed status
3. **Optimistic Concurrency**: Using `resourceVersion` for safe updates

### Client Responsibilities

Clients creating resources:

1. **MAY omit status**: Server will populate initial status
2. **MAY provide status**: Server will preserve client-provided phase if set
3. **MUST use status subresource**: For status-only updates, use `/status` endpoint

---

## Condition Types

### Standard Conditions

| Type | Used By | Meaning |
|------|---------|---------|
| `Accepted` | Goal, Execution | Resource has been accepted by the system |
| `Valid` | Plan | Resource passed schema validation |
| `Available` | Capability | Capability is available for use |
| `Resolved` | Binding | All capability references have been resolved |
| `Progressing` | Any | Resource is making progress toward desired state |
| `Ready` | Any | Resource is fully ready |

### Condition Status Values

- `True`: Condition is satisfied
- `False`: Condition is not satisfied
- `Unknown`: Condition status cannot be determined

---

## Examples

### Creating a Goal

```yaml
# Request (status omitted)
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: my-goal
  namespace: default
spec:
  description: "Implement feature X"
  acceptanceCriteria:
    - description: "Tests pass"
  priority: 100

# Response (status auto-populated)
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: my-goal
  namespace: default
  uid: abc-123
  resourceVersion: "1"
  generation: 1
spec:
  description: "Implement feature X"
  acceptanceCriteria:
    - description: "Tests pass"
  priority: 100
status:
  phase: Pending
  observedGeneration: 1
  conditions:
    - type: Accepted
      status: "True"
      reason: GoalCreated
      message: "Goal has been accepted"
      lastTransitionTime: "2024-01-15T10:00:00Z"
      observedGeneration: 1
```

### Updating Goal Phase (Controller)

```yaml
# Controller updates status via /status endpoint
PUT /apis/planspec.io/v1alpha1/namespaces/default/goals/my-goal/status

status:
  phase: Ready
  observedGeneration: 1
  conditions:
    - type: Accepted
      status: "True"
      reason: GoalCreated
      message: "Goal has been accepted"
      lastTransitionTime: "2024-01-15T10:00:00Z"
    - type: PlanSelected
      status: "True"
      reason: PlanApproved
      message: "Plan 'my-goal-plan-v1' selected"
      lastTransitionTime: "2024-01-15T10:05:00Z"
  activePlanRef:
    name: my-goal-plan-v1
```
