#!/usr/bin/env bash
# validate.sh — Validate PlanSpec YAML files
# Usage: ./validate.sh <file-or-directory>
#
# Checks:
# - Valid YAML
# - Correct apiVersion (planspec.io/v1alpha1)
# - Required fields (kind, metadata.name)
# - Node ID uniqueness (for Plans)
# - Edge reference validity (for Plans)

set -euo pipefail

PLANSPEC_SERVER="${PLANSPEC_SERVER:-http://localhost:9090}"
PLANSPEC_CLI="${PLANSPEC_CLI:-/home/node/.openclaw/workspace/planspec/target/debug/planspec}"

usage() {
  echo "Usage: $0 <file-or-directory>"
  echo "Validates PlanSpec YAML files for correctness."
  exit 1
}

[[ $# -lt 1 ]] && usage

TARGET="$1"
ERRORS=0
CHECKED=0

validate_file() {
  local file="$1"
  local basename
  basename=$(basename "$file")

  # Skip non-YAML files
  case "$file" in
    *.yaml|*.yml|*.json) ;;
    *) return ;;
  esac

  CHECKED=$((CHECKED + 1))

  # Check valid YAML
  if ! python3 -c "import yaml, sys; yaml.safe_load(open(sys.argv[1]))" "$file" 2>/dev/null; then
    echo "❌ $basename: Invalid YAML"
    ERRORS=$((ERRORS + 1))
    return
  fi

  # Check apiVersion
  local api_version
  api_version=$(python3 -c "import yaml, sys; d=yaml.safe_load(open(sys.argv[1])); print(d.get('apiVersion',''))" "$file" 2>/dev/null)
  if [[ "$api_version" != "planspec.io/v1alpha1" ]]; then
    echo "❌ $basename: Wrong apiVersion '$api_version' (expected planspec.io/v1alpha1)"
    ERRORS=$((ERRORS + 1))
    return
  fi

  # Check required fields
  local kind name
  kind=$(python3 -c "import yaml, sys; d=yaml.safe_load(open(sys.argv[1])); print(d.get('kind',''))" "$file" 2>/dev/null)
  name=$(python3 -c "import yaml, sys; d=yaml.safe_load(open(sys.argv[1])); print(d.get('metadata',{}).get('name',''))" "$file" 2>/dev/null)

  if [[ -z "$kind" ]]; then
    echo "❌ $basename: Missing 'kind'"
    ERRORS=$((ERRORS + 1))
    return
  fi

  if [[ -z "$name" ]]; then
    echo "❌ $basename: Missing 'metadata.name'"
    ERRORS=$((ERRORS + 1))
    return
  fi

  # For Plans: check node ID uniqueness and edge validity
  if [[ "$kind" == "Plan" ]]; then
    python3 -c "
import yaml, sys, collections

d = yaml.safe_load(open(sys.argv[1]))
graph = d.get('spec', {}).get('graph', {})
nodes = graph.get('nodes', [])
edges = graph.get('edges', [])

# Collect all node IDs (including children of groups)
def collect_ids(nodes):
    ids = []
    for n in nodes:
        ids.append(n.get('id', ''))
        for child in n.get('children', []):
            if isinstance(child, str):
                ids.append(child)
            elif isinstance(child, dict):
                ids.append(child.get('id', ''))
    return ids

all_ids = collect_ids(nodes)
node_set = set(all_ids)

# Check duplicates
dupes = [id for id, count in collections.Counter(all_ids).items() if count > 1 and id]
if dupes:
    print(f'❌ {sys.argv[2]}: Duplicate node IDs: {dupes}')
    sys.exit(1)

# Check edge references
for e in edges:
    frm = e.get('from', '')
    to = e.get('to', '')
    if frm and frm not in node_set:
        print(f'❌ {sys.argv[2]}: Edge references unknown node: {frm}')
        sys.exit(1)
    if to and to not in node_set:
        print(f'❌ {sys.argv[2]}: Edge references unknown node: {to}')
        sys.exit(1)

print(f'✅ {sys.argv[2]}: Valid {d[\"kind\"]} \"{name}\" ({len(nodes)} nodes, {len(edges)} edges)')
" "$file" "$basename" 2>/dev/null || ERRORS=$((ERRORS + 1))
    return
  fi

  echo "✅ $basename: Valid $kind \"$name\""
}

# If CLI is available, try using it first
if [[ -x "$PLANSPEC_CLI" ]]; then
  if PLANSPEC_SERVER="$PLANSPEC_SERVER" "$PLANSPEC_CLI" validate -f "$TARGET" 2>/dev/null; then
    echo "✅ CLI validation passed for $TARGET"
    exit 0
  fi
  echo "⚠️  CLI validation unavailable, falling back to structural checks"
fi

# Manual validation
if [[ -d "$TARGET" ]]; then
  for f in "$TARGET"/*.yaml "$TARGET"/*.yml "$TARGET"/*.json; do
    [[ -f "$f" ]] && validate_file "$f"
  done
elif [[ -f "$TARGET" ]]; then
  validate_file "$TARGET"
else
  echo "❌ Not found: $TARGET"
  exit 1
fi

echo ""
echo "Checked $CHECKED files, $ERRORS errors"
[[ $ERRORS -eq 0 ]] && exit 0 || exit 1
