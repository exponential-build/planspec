---
name: planspec
description: Create, convert, and execute PlanSpec plans. Use for planning tasks as PlanSpec Goal+Plan resources, converting markdown plans to PlanSpec, or implementing existing plans. Requires planspec server to be running.
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite, Task, WebFetch, AskUserQuestion
---

# PlanSpec Skill

Work orchestration using PlanSpec Goal, Plan, and Execution resources.

## Commands

| Command | Description |
|---------|-------------|
| `/planspec setup` | Install CLI and configure for this project |
| `/planspec server` | Start the PlanSpec server |
| `/planspec plan <task>` | Create a Goal + Plan for a task |
| `/planspec convert [file]` | Convert a markdown plan to PlanSpec |
| `/planspec implement <plan>` | Execute a plan, tracking progress |
| `/planspec status [plan]` | Show execution status |

## Configuration

Configuration is stored in `.planspec.yaml` in the project root:

```yaml
# .planspec.yaml
server:
  url: http://localhost:8080

paths:
  goals: planspec/goals
  plans: planspec/plans
  capabilities: planspec/capabilities
  bindings: planspec/bindings

namespace: default

cli:
  path: planspec  # or absolute path to binary
```

**Defaults** (if no config file):
- `server.url`: `http://localhost:8080`
- `paths.*`: `planspec/goals`, `planspec/plans`, etc.
- `namespace`: `default`
- `cli.path`: `planspec`

---

## /planspec setup

**Usage:** `/planspec setup`

Initialize PlanSpec for this project.

### Process

1. **Check for existing config**: Look for `.planspec.yaml`

2. **Detect or install CLI**:
   ```bash
   # Check if planspec is available
   which planspec || echo "not found"
   ```

   If not found, offer options:
   - Install from crates.io: `cargo install planspec`
   - Clone and build: `git clone ... && cd planspec/cli && cargo build --release`
   - Use local path if in planspec repo

3. **Create directory structure**:
   ```bash
   mkdir -p planspec/{goals,plans,capabilities,bindings,executions}
   ```

4. **Ask for configuration**:
   - Server URL (default: http://localhost:8080)
   - Namespace (default: default)
   - Custom paths? (default: planspec/*)

5. **Write `.planspec.yaml`**:
   ```yaml
   server:
     url: http://localhost:8080
   paths:
     goals: planspec/goals
     plans: planspec/plans
     capabilities: planspec/capabilities
     bindings: planspec/bindings
   namespace: default
   cli:
     path: planspec
   ```

6. **Create default capabilities** (optional):
   Ask if user wants to create standard capability definitions.

7. **Verify setup**:
   ```bash
   planspec --version
   curl -s http://localhost:8080/health
   ```

---

## /planspec server

**Usage:** `/planspec server [start|stop|status]`

Manage the PlanSpec server.

### Process

1. **Read config** from `.planspec.yaml`

2. **For `start`**:
   ```bash
   # Check if already running
   curl -s $SERVER_URL/health && echo "Already running" && exit 0

   # Start server
   # If in planspec repo:
   cd server && cargo run --release &

   # If installed globally:
   planspec-server --port 8080 &

   # Wait for ready
   for i in {1..30}; do
     curl -s $SERVER_URL/health && break
     sleep 1
   done
   ```

3. **For `status`**:
   ```bash
   curl -s $SERVER_URL/health && echo "Running" || echo "Not running"
   ```

4. **For `stop`**:
   ```bash
   pkill -f planspec-server
   ```

---

## /planspec plan

**Usage:** `/planspec plan <task description>`

Create a new Goal and Plan for a task.

### Prerequisites
- Server must be running: `curl -s $SERVER_URL/health`
- If not running, prompt: "Server not running. Start with `/planspec server start`"

### Process

1. **Load config** from `.planspec.yaml`

2. **Verify server is running**:
   ```bash
   curl -s $(yq '.server.url' .planspec.yaml)/health || {
     echo "Server not running. Use /planspec server start"
     exit 1
   }
   ```

3. **Analyze the task** - Understand what needs to be done

4. **Explore if needed** - Use Glob/Grep/Read to understand relevant code

5. **Ask clarifying questions** - Use AskUserQuestion for ambiguous requirements

6. **Generate Goal name** from task (lowercase, hyphenated)

7. **Create Goal file** at `$PATHS_GOALS/{name}.yaml`:
   ```yaml
   apiVersion: planspec.io/v1alpha1
   kind: Goal
   metadata:
     name: {name}
     namespace: {namespace}
   spec:
     description: "{task description}"
     acceptanceCriteria:
       - description: "{criterion 1}"
       - description: "{criterion 2}"
     priority: 100
   ```

8. **Create Plan file** at `$PATHS_PLANS/{name}-v1.yaml`:
   ```yaml
   apiVersion: planspec.io/v1alpha1
   kind: Plan
   metadata:
     name: {name}-v1
     namespace: {namespace}
     labels:
       planspec.io/goal: {name}
       planspec.io/series: {name}
       planspec.io/version: "1"
   spec:
     goalRef:
       name: {name}
     series: {name}
     version: "1"
     description: "Implementation plan for {task}"
     graph:
       nodes: [...]
       edges: [...]
   ```

9. **Validate locally**:
   ```bash
   $CLI validate -f $PATHS_GOALS/{name}.yaml
   $CLI validate -f $PATHS_PLANS/{name}-v1.yaml
   ```

10. **Apply to server**:
    ```bash
    $CLI apply -f $PATHS_GOALS/{name}.yaml
    $CLI apply -f $PATHS_PLANS/{name}-v1.yaml
    ```

11. **Confirm creation**:
    ```bash
    $CLI get goal {name} -n {namespace}
    $CLI get plan {name}-v1 -n {namespace}
    ```

### Node Types
- **Task** - Work to be done (has capabilityRef, outputs, context)
- **Gate** - Human approval point (when: "manualApproval")
- **External** - Dependency on another goal/system
- **Group** - Logical grouping of nodes

### Example Output
```
Created: plans/goals/add-health-endpoint.yaml
Created: plans/plans/add-health-endpoint-v1.yaml

Applied to server:
  Goal/add-health-endpoint created
  Plan/add-health-endpoint-v1 created

To implement: /planspec implement add-health-endpoint-v1
```

---

## /planspec convert

**Usage:** `/planspec convert [path-to-markdown-plan]`

Convert a markdown plan to PlanSpec resources.

### Prerequisites
- Server must be running

### Process

1. **Load config** and verify server

2. **Read the markdown plan**:
   - If path provided, read that file
   - If no path, check `~/.claude/plans/` for recent plans
   - Ask user to select if multiple found

3. **Parse markdown structure**:
   - Title → Goal name
   - Objective section → Goal description
   - Requirements/Criteria → acceptanceCriteria
   - Steps/Implementation → Plan nodes
   - Dependencies mentioned → edges

4. **Generate Goal and Plan YAML**

5. **Validate and apply to server**

6. **Report created resources**

### Markdown Patterns
```markdown
# Title                           → Goal name
## Objective                      → Goal description
## Acceptance Criteria            → acceptanceCriteria
- [ ] criterion                   → acceptanceCriteria item
## Steps / ## Implementation      → Plan nodes
### Step 1: Name                  → Node
- depends on Step N               → Edge
```

---

## /planspec implement

**Usage:** `/planspec implement <plan-name> [-n namespace]`

Execute a plan by working through its nodes.

### Prerequisites
- Server must be running
- Plan must exist in server

### Process

1. **Load config**:
   ```bash
   SERVER_URL=$(yq '.server.url' .planspec.yaml)
   NAMESPACE=$(yq '.namespace' .planspec.yaml)
   CLI=$(yq '.cli.path' .planspec.yaml)
   ```

2. **Verify server**:
   ```bash
   curl -s $SERVER_URL/health || {
     echo "Server not running. Use /planspec server start"
     exit 1
   }
   ```

3. **Load Plan from server**:
   ```bash
   $CLI get plan $PLAN_NAME -n $NAMESPACE -o yaml
   ```

4. **Get Goal reference** from plan's `spec.goalRef`

5. **Find next execution number**:
   ```bash
   $CLI get executions -n $NAMESPACE -o yaml | grep "name: $PLAN_NAME-exec" | wc -l
   ```

6. **Create Execution**:
   ```bash
   cat <<EOF | $CLI apply -f -
   apiVersion: planspec.io/v1alpha1
   kind: Execution
   metadata:
     name: $PLAN_NAME-exec-$(printf "%03d" $NEXT_NUM)
     namespace: $NAMESPACE
   spec:
     goalRef:
       name: $GOAL_NAME
     planRef:
       name: $PLAN_NAME
   EOF
   ```

7. **Topologically sort nodes** by edges

8. **Create todos** from nodes in execution order using TodoWrite

9. **Execute each node**:

   For each node in order:

   a. Mark todo `in_progress`

   b. Read node's `context` for implementation guidance

   c. Perform the work:
      - **Task**: Do the work described, produce outputs
      - **Gate**: Pause, ask user for approval with AskUserQuestion
      - **External**: Check if dependency is satisfied

   d. Update execution status:
      ```bash
      # Note: status update endpoint may vary by server implementation
      $CLI get execution $EXEC_NAME -n $NAMESPACE -o yaml | \
        yq '.status.nodeStatuses["'$NODE_ID'"] = {"phase": "Completed"}' | \
        $CLI apply -f -
      ```

   e. Mark todo `completed`

10. **Complete execution**:
    ```bash
    $CLI get execution $EXEC_NAME -n $NAMESPACE -o yaml | \
      yq '.status.phase = "Completed"' | \
      $CLI apply -f -
    ```

### Example Session
```
> /planspec implement add-health-endpoint-v1

Loading plan from server...
Plan: add-health-endpoint-v1 (5 nodes)
Goal: add-health-endpoint

Creating execution: add-health-endpoint-v1-exec-001

Execution order:
  1. [Task] design-endpoint
  2. [Task] implement-handler
  3. [Task] add-tests
  4. [Task] update-docs
  5. [Gate] review

Starting node 1/5: design-endpoint
  Context: Design a /health endpoint that returns...
  Working...
  ✓ Completed

Starting node 2/5: implement-handler
  ...
```

---

## /planspec status

**Usage:** `/planspec status [plan-name]`

Show execution status.

### Process

1. **Load config** and verify server

2. **Query executions**:
   ```bash
   # All executions
   $CLI get executions -n $NAMESPACE

   # For specific plan
   $CLI get executions -n $NAMESPACE | grep $PLAN_NAME
   ```

3. **Get detailed status**:
   ```bash
   $CLI describe execution $EXEC_NAME -n $NAMESPACE
   ```

4. **Display progress**:
   ```
   Plan: add-health-endpoint-v1
   Execution: add-health-endpoint-v1-exec-001
   Phase: Running
   Progress: 3/5 nodes (60%)

   Nodes:
     ✓ design-endpoint      Completed
     ✓ implement-handler    Completed
     ✓ add-tests            Completed
     ○ update-docs          Pending
     ◇ review               Gate (waiting)
   ```

---

## Capabilities Reference

Standard capabilities to reference in plan nodes:

| Capability | Use For |
|------------|---------|
| `code-generation` | Writing new code |
| `code-analysis` | Understanding existing code |
| `code-review` | Reviewing changes |
| `refactoring` | Restructuring code |
| `test-generation` | Writing tests |
| `test-execution` | Running tests |
| `documentation` | Writing docs |
| `research` | Investigating options |
| `planning` | Creating plans |
| `architecture` | Design decisions |

---

## Troubleshooting

### Server not running
```
Error: Server not running at http://localhost:8080
```
Fix: `/planspec server start`

### CLI not found
```
Error: planspec command not found
```
Fix: `/planspec setup` to install

### Config not found
```
Warning: No .planspec.yaml found, using defaults
```
Fix: `/planspec setup` to create config

---

## Files

```
.planspec.yaml              # Project configuration
planspec/
├── goals/                  # Goal resources
├── plans/                  # Plan resources
├── capabilities/           # Capability definitions
├── bindings/              # Binding rules
└── executions/            # (optional) execution records
```
