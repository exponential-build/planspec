# PlanSpec Examples

Complete, copy-paste-ready YAML examples for all resource types. Apply with `planspec apply -f <file>` or `curl -X POST`.

## Goal

```yaml
apiVersion: planspec.io/v1alpha1
kind: Goal
metadata:
  name: deploy-service
  namespace: default
spec:
  description: |
    Deploy the payment service to production with zero downtime.
    Includes database migration, canary rollout, and monitoring setup.
  acceptanceCriteria:
    - name: service-healthy
      type: endpoint_responds
      url: https://payments.example.com/health
      expectedStatus: 200
    - name: latency-ok
      type: custom
      description: P99 latency under 200ms for 24 hours post-deploy
      validator: manual-verification
```

**Apply:**
```bash
planspec apply -f goal.yaml
# or
curl -X POST http://localhost:9090/apis/planspec.io/v1alpha1/namespaces/default/goals \
  -H 'Content-Type: application/json' \
  -d @goal.json
```

## Plan with Tasks, Gates, and Edges

```yaml
apiVersion: planspec.io/v1alpha1
kind: Plan
metadata:
  name: deploy-service-v1
  namespace: default
  labels:
    planspec.io/goal: deploy-service
    planspec.io/series: deploy-service
    planspec.io/version: "1"
spec:
  description: Canary deployment plan with review gates
  goalRef:
    name: deploy-service
  series: deploy-service
  version: "1"
  context:
    - name: deployment-notes
      format: markdown
      content: |
        Current version: v2.3.1
        Target version: v2.4.0
        Database migration: yes (backward compatible)
  graph:
    nodes:
      - id: plan-approval
        kind: Gate
        description: Approve the deployment plan
        gateRef:
          name: deploy-service-plan-approval

      - id: db-migration
        kind: Task
        description: Run database migration (backward compatible)
        estimatedEffort: small
        capabilityRefs:
          - name: devops
        acceptanceCriteria:
          - name: migration-applied
            type: command_succeeds
            command: ./migrate
            args: ["status"]
            description: Migration status shows all applied
        timeout: 1h

      - id: canary-deploy
        kind: Task
        description: Deploy canary (10% traffic)
        estimatedEffort: medium
        capabilityRefs:
          - name: devops
        inputs:
          target_version: v2.4.0
          canary_weight: "10"
        outputs:
          - canary_deployment_url
        acceptanceCriteria:
          - name: canary-healthy
            type: endpoint_responds
            url: https://payments-canary.example.com/health
            expectedStatus: 200
          - name: error-rate-ok
            type: custom
            description: Error rate < 0.1% for 30 minutes
            validator: manual-verification
        timeout: 2h

      - id: canary-review
        kind: Gate
        description: Review canary metrics before full rollout
        gateRef:
          name: deploy-service-canary-review

      - id: full-rollout
        kind: Task
        description: Promote canary to 100% traffic
        estimatedEffort: small
        capabilityRefs:
          - name: devops
        acceptanceCriteria:
          - name: rollout-complete
            type: endpoint_responds
            url: https://payments.example.com/health
            expectedStatus: 200
        timeout: 1h

    edges:
      - from: plan-approval
        to: db-migration
        type: hard
      - from: db-migration
        to: canary-deploy
        type: hard
      - from: canary-deploy
        to: canary-review
        type: hard
      - from: canary-review
        to: full-rollout
        type: hard
```

## Gate (All 3 Types)

### Approval Gate

```yaml
apiVersion: planspec.io/v1alpha1
kind: Gate
metadata:
  name: deploy-service-plan-approval
  namespace: default
  labels:
    planspec.io/goal: deploy-service
spec:
  gateType: approval
  description: Approve the deployment plan before work begins
  reviewers:
    - user:alice
  targetRef:
    kind: Plan
    name: deploy-service-v1
    nodeId: plan-approval
```

### Review Gate

```yaml
apiVersion: planspec.io/v1alpha1
kind: Gate
metadata:
  name: deploy-service-canary-review
  namespace: default
  labels:
    planspec.io/goal: deploy-service
spec:
  gateType: review
  description: |
    Review canary deployment metrics:
    - Error rate < 0.1%
    - P99 latency < 200ms
    - No increase in 5xx responses
  reviewers:
    - user:alice
    - user:bob
  requiredApprovers: 1
  targetRef:
    kind: Plan
    name: deploy-service-v1
    nodeId: canary-review
```

### Sign-Off Gate

```yaml
apiVersion: planspec.io/v1alpha1
kind: Gate
metadata:
  name: compliance-sign-off
  namespace: default
spec:
  gateType: sign-off
  description: Security team signs off on production deployment
  reviewers:
    - user:security-lead
  requiredApprovers: 1
  targetRef:
    kind: Plan
    name: deploy-service-v1
    nodeId: compliance-gate
```

## Capability

```yaml
apiVersion: planspec.io/v1alpha1
kind: Capability
metadata:
  name: devops
  namespace: default
spec:
  description: Infrastructure and deployment operations
  skills:
    - kubernetes
    - helm
    - ci-cd
    - monitoring
  tools:
    - kubectl
    - helm
    - gh
```

## Binding

```yaml
apiVersion: planspec.io/v1alpha1
kind: Binding
metadata:
  name: athena-devops
  namespace: default
spec:
  agentRef:
    name: athena
  capabilityRef:
    name: devops
  constraints:
    maxConcurrency: 2
    environments:
      - staging
      - production
```

## Execution

```yaml
apiVersion: planspec.io/v1alpha1
kind: Execution
metadata:
  name: deploy-service-run-1
  namespace: default
  labels:
    planspec.io/goal: deploy-service
spec:
  goalRef:
    name: deploy-service
    namespace: default
  planRef:
    name: deploy-service-v1
    namespace: default
  context:
    - name: run-notes
      format: markdown
      content: |
        Production deployment initiated 2026-02-15.
        Deployer: Athena
        Approval: pending from Alice
```

**Apply:**
```bash
planspec apply -f execution.yaml
```

## Acceptance Criteria Types

All 5 types with their specific fields:

### artifact_exists

```yaml
acceptanceCriteria:
  - name: dockerfile-exists
    type: artifact_exists
    description: Dockerfile exists in the repo
    path: agent-containers/Dockerfile
    contentMatch: "FROM.*openclaw"
```

### endpoint_responds

```yaml
acceptanceCriteria:
  - name: api-healthy
    type: endpoint_responds
    url: http://localhost:8080/health
    expectedStatus: 200
    bodyMatch: "ok"
    description: API health endpoint returns 200
```

### command_succeeds

```yaml
acceptanceCriteria:
  - name: tests-pass
    type: command_succeeds
    command: npm
    args: ["test", "--", "--ci"]
    description: All tests pass
    outputMatch: "passed"
```

### test_passes

```yaml
acceptanceCriteria:
  - name: unit-tests
    type: test_passes
    framework: jest
    testPattern: "src/**/*.test.ts"
    description: All unit tests pass
```

### custom

```yaml
acceptanceCriteria:
  - name: manual-check
    type: custom
    description: Verified by human inspection
    validator: manual-verification
```

## Plan Versioning (series/version/supersedes)

### Version 1 (original)

```yaml
apiVersion: planspec.io/v1alpha1
kind: Plan
metadata:
  name: my-plan-v1
  labels:
    planspec.io/series: my-plan
    planspec.io/version: "1"
spec:
  series: my-plan
  version: "1"
  goalRef:
    name: my-goal
  graph:
    nodes:
      - id: step-1
        kind: Task
        description: Original approach
```

### Version 2 (supersedes v1)

```yaml
apiVersion: planspec.io/v1alpha1
kind: Plan
metadata:
  name: my-plan-v2
  labels:
    planspec.io/series: my-plan
    planspec.io/version: "2"
spec:
  series: my-plan
  version: "2"
  supersedes:
    - name: my-plan-v1
  goalRef:
    name: my-goal
  graph:
    nodes:
      - id: step-1
        kind: Task
        description: Revised approach with improvements
```

## Context Attachments

```yaml
spec:
  context:
    # Inline markdown
    - name: design-notes
      format: markdown
      content: |
        ## Key Decisions
        - Use canary deployment
        - Database migration first

    # URI references
    - name: reference-docs
      format: uri-list
      uris:
        - https://docs.example.com/deploy-guide
        - file:///workspace/docs/runbook.md
```
