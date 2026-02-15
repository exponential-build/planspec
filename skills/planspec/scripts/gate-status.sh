#!/usr/bin/env bash
# gate-status.sh — Quick overview of all gates
# Usage: ./gate-status.sh [namespace]
# If no namespace given, queries all namespaces.

set -euo pipefail

PLANSPEC_SERVER="${PLANSPEC_SERVER:-http://localhost:9090}"
NS="${1:-}"

if [[ -n "$NS" ]]; then
  URL="$PLANSPEC_SERVER/apis/planspec.io/v1alpha1/namespaces/$NS/gates"
else
  URL="$PLANSPEC_SERVER/apis/planspec.io/v1alpha1/gates"
fi

echo "Gates from $URL"
echo "─────────────────────────────────────────────────────────────"

curl -sf "$URL" | python3 -c "
import json, sys
from datetime import datetime, timezone

data = json.load(sys.stdin)
items = data.get('items', [])

if not items:
    print('No gates found.')
    sys.exit(0)

# Column widths
fmt = '{:<40} {:<12} {:<10} {:<12} {}'
print(fmt.format('NAME', 'NAMESPACE', 'TYPE', 'PHASE', 'REVIEWERS'))
print('─' * 100)

for g in sorted(items, key=lambda x: x['metadata'].get('namespace', '') + x['metadata']['name']):
    name = g['metadata']['name']
    ns = g['metadata'].get('namespace', 'default')
    gate_type = g['spec'].get('gateType', '?')
    phase = g.get('status', {}).get('phase', 'No-Status')
    reviewers = ', '.join(g['spec'].get('reviewers', []))

    # Color coding
    if phase == 'Approved':
        phase_display = '✅ ' + phase
    elif phase == 'Rejected':
        phase_display = '❌ ' + phase
    elif phase in ('Pending', 'Waiting'):
        phase_display = '⏳ ' + phase
    else:
        phase_display = '◻️  ' + phase

    print(fmt.format(name[:40], ns[:12], gate_type[:10], phase_display, reviewers))

# Summary
approved = sum(1 for g in items if g.get('status', {}).get('phase') == 'Approved')
pending = sum(1 for g in items if g.get('status', {}).get('phase') in (None, '', 'Pending', 'Waiting') or 'phase' not in g.get('status', {}))
rejected = sum(1 for g in items if g.get('status', {}).get('phase') == 'Rejected')

print()
print(f'Total: {len(items)} | ✅ Approved: {approved} | ⏳ Pending: {pending} | ❌ Rejected: {rejected}')
"
