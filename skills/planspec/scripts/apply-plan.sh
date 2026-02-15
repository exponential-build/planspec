#!/usr/bin/env bash
# apply-plan.sh — Apply a plan and its gate resources in the correct order
# Usage: ./apply-plan.sh <plan.yaml> [gate1.yaml gate2.yaml ...] [--exec]
#
# If --exec is passed, also creates an execution for the plan.
# Gates are applied first (plans reference them via gateRef).

set -euo pipefail

PLANSPEC_SERVER="${PLANSPEC_SERVER:-http://localhost:9090}"
PLANSPEC_CLI="${PLANSPEC_CLI:-/home/node/.openclaw/workspace/planspec/target/debug/planspec}"

usage() {
  echo "Usage: $0 <plan.yaml> [gate1.yaml gate2.yaml ...] [--exec]"
  echo ""
  echo "Applies PlanSpec resources in the correct order:"
  echo "  1. Gate resources (referenced by the plan)"
  echo "  2. The plan itself"
  echo "  3. (Optional) An execution if --exec is passed"
  echo ""
  echo "Options:"
  echo "  --exec    Also create an execution after applying the plan"
  exit 1
}

[[ $# -lt 1 ]] && usage

PLAN_FILE=""
GATE_FILES=()
CREATE_EXEC=false

for arg in "$@"; do
  case "$arg" in
    --exec) CREATE_EXEC=true ;;
    --help|-h) usage ;;
    *)
      if [[ -z "$PLAN_FILE" ]]; then
        PLAN_FILE="$arg"
      else
        GATE_FILES+=("$arg")
      fi
      ;;
  esac
done

[[ -z "$PLAN_FILE" ]] && usage
[[ ! -f "$PLAN_FILE" ]] && echo "❌ File not found: $PLAN_FILE" && exit 1

apply() {
  local file="$1"
  local kind
  kind=$(python3 -c "import yaml; print(yaml.safe_load(open('$file')).get('kind','?'))" 2>/dev/null)
  local name
  name=$(python3 -c "import yaml; print(yaml.safe_load(open('$file'))['metadata']['name'])" 2>/dev/null)

  echo -n "Applying $kind/$name... "

  if [[ -x "$PLANSPEC_CLI" ]]; then
    if PLANSPEC_SERVER="$PLANSPEC_SERVER" "$PLANSPEC_CLI" apply -f "$file" 2>/dev/null; then
      echo "✅"
      return 0
    fi
  fi

  # Fallback to curl
  local ns
  ns=$(python3 -c "import yaml; print(yaml.safe_load(open('$file'))['metadata'].get('namespace','default'))" 2>/dev/null)
  local resource
  resource=$(python3 -c "
import yaml
kind = yaml.safe_load(open('$file'))['kind'].lower()
# Pluralize
if kind.endswith('y'): print(kind[:-1] + 'ies')
else: print(kind + 's')
" 2>/dev/null)

  local status_code
  status_code=$(curl -s -o /dev/null -w '%{http_code}' \
    -X POST "$PLANSPEC_SERVER/apis/planspec.io/v1alpha1/namespaces/$ns/$resource" \
    -H 'Content-Type: application/json' \
    -d "$(python3 -c 'import yaml,json,sys; print(json.dumps(yaml.safe_load(open(sys.argv[1]))))' "$file")")

  if [[ "$status_code" == "201" || "$status_code" == "200" ]]; then
    echo "✅"
  elif [[ "$status_code" == "409" ]]; then
    echo "⚠️  Already exists (409)"
  else
    echo "❌ HTTP $status_code"
    return 1
  fi
}

# Step 1: Apply gates
if [[ ${#GATE_FILES[@]} -gt 0 ]]; then
  echo "=== Applying Gates ==="
  for gf in "${GATE_FILES[@]}"; do
    [[ -f "$gf" ]] && apply "$gf"
  done
  echo ""
fi

# Step 2: Apply plan
echo "=== Applying Plan ==="
apply "$PLAN_FILE"
echo ""

# Step 3: Optionally create execution
if [[ "$CREATE_EXEC" == true ]]; then
  echo "=== Creating Execution ==="
  PLAN_NAME=$(python3 -c "import yaml; print(yaml.safe_load(open('$PLAN_FILE'))['metadata']['name'])" 2>/dev/null)
  PLAN_NS=$(python3 -c "import yaml; print(yaml.safe_load(open('$PLAN_FILE'))['metadata'].get('namespace','default'))" 2>/dev/null)
  GOAL_NAME=$(python3 -c "import yaml; print(yaml.safe_load(open('$PLAN_FILE'))['spec'].get('goalRef',{}).get('name',''))" 2>/dev/null)

  EXEC_NAME="${PLAN_NAME}-run-1"
  echo -n "Creating Execution/$EXEC_NAME... "

  EXEC_JSON=$(python3 -c "
import json
print(json.dumps({
    'apiVersion': 'planspec.io/v1alpha1',
    'kind': 'Execution',
    'metadata': {
        'name': '$EXEC_NAME',
        'namespace': '$PLAN_NS',
        'labels': {'planspec.io/goal': '$GOAL_NAME'} if '$GOAL_NAME' else {}
    },
    'spec': {
        'planRef': {'name': '$PLAN_NAME', 'namespace': '$PLAN_NS'},
        **(({'goalRef': {'name': '$GOAL_NAME', 'namespace': '$PLAN_NS'}}) if '$GOAL_NAME' else {})
    }
}))
")

  STATUS=$(curl -s -o /dev/null -w '%{http_code}' \
    -X POST "$PLANSPEC_SERVER/apis/planspec.io/v1alpha1/namespaces/$PLAN_NS/executions" \
    -H 'Content-Type: application/json' \
    -d "$EXEC_JSON")

  if [[ "$STATUS" == "201" || "$STATUS" == "200" ]]; then
    echo "✅"
  else
    echo "❌ HTTP $STATUS"
  fi
fi

echo ""
echo "Done."
