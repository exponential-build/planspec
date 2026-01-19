# PlanSpec Fixtures

This directory contains versioned bundles of PlanSpec resources for testing and development.

## Directory Structure

```
fixtures/
├── v0.1.0/
│   ├── bundle.yaml         # All resources concatenated (apply this)
│   ├── capabilities.yaml   # Capability definitions
│   ├── goals.yaml          # Goal definitions
│   ├── plans.yaml          # Plan definitions
│   └── bindings.yaml       # Binding definitions
└── README.md
```

## Usage

### Apply All Resources

```bash
# Start fresh
rm -f planspec.db server/planspec.db
cd server && cargo run &

# Apply the fixture bundle
planspec apply -f fixtures/v0.1.0/bundle.yaml
```

### Apply Individually

```bash
planspec apply -f fixtures/v0.1.0/capabilities.yaml
planspec apply -f fixtures/v0.1.0/goals.yaml
planspec apply -f fixtures/v0.1.0/plans.yaml
planspec apply -f fixtures/v0.1.0/bindings.yaml
```

### Verify Resources

```bash
planspec get goals -n planspec           # Should show 9 goals
planspec get capabilities -n planspec    # Should show 12 capabilities
planspec get plans -n planspec           # Should show 7 plans
planspec get bindings -n planspec        # Should show 1 binding
```

## Version History

### v0.1.0

Initial fixture version containing:

**Goals (9):**
- `planspec-v0-ready-for-exponential` - Meta-goal for v0 readiness
- `planspec-schema-and-conventions-v0` - Schema conventions
- `planspec-fixtures-and-integration-tests` - Test infrastructure
- `planspec-goal-to-plan-resolution-mvp` - Plan resolution
- `planspec-status-and-observability-v0` - Status and observability
- `planspec-core-crate` - Shared core crate
- `implement-apiserver-lite` - API server implementation
- `cli-improvements` - CLI improvements
- `add-context-fields` - Context attachment fields

**Capabilities (12):**
- Development: `code-analysis`, `code-generation`, `code-review`, `refactoring`
- Testing: `test-generation`, `test-execution`
- Documentation: `documentation`
- Analysis: `research`, `security-scan`
- Planning: `planning`, `architecture`

**Plans (7):**
- `planspec-v0-goal-graph` - Goal dependency graph
- `planspec-core-v1` - Core crate implementation
- `apiserver-lite-v1`, `apiserver-lite-v2` - Server implementation
- `cli-improvements-v1`, `cli-improvements-v2` - CLI improvements
- `add-context-fields-v1` - Context fields implementation

**Bindings (1):**
- `default-bindings` - Maps all capabilities to claude-code agent

## Creating New Versions

1. Create a new version directory: `mkdir fixtures/v0.2.0`
2. Copy and modify files from the previous version
3. Update this README with the new version's contents
4. Regenerate bundle.yaml: `cat capabilities.yaml goals.yaml plans.yaml bindings.yaml > bundle.yaml`
