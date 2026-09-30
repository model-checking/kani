#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that the export aggregates multiple harnesses consistently.

set -eu
set -o pipefail

OUTPUT_FILE="multi_harness_output.json"
trap 'rm -f "$OUTPUT_FILE"' EXIT

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"

kani -Z export-json test.rs --export-json "$OUTPUT_FILE"

if [ ! -f "$OUTPUT_FILE" ]; then
    echo "ERROR: JSON file $OUTPUT_FILE was not created"
    exit 1
fi

python3 "$VALIDATOR" "$OUTPUT_FILE" 2>&1 | tail -1

python3 << EOF
import json
import sys

with open('$OUTPUT_FILE', 'r') as f:
    data = json.load(f)

expected = {
    'verify_multiply_positive',
    'verify_multiply_zero',
    'verify_divide_nonzero',
}

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


harnesses = data['harnesses']
check(len(harnesses) == 3, f"expected 3 harnesses, got {len(harnesses)}")
names = {h.get('name') for h in harnesses}
check(names == expected,
      f"harnesses should cover {sorted(expected)}, got {sorted(map(str, names))}")
check(all(h.get('crate_name') for h in harnesses),
      "every harness should report a non-empty crate_name")

summary = data['summary']
for field, want in [('total', 3), ('successful', 3), ('failed', 0)]:
    check(summary[field] == want,
          f"summary.{field} should be {want}, got {summary[field]}")
check(data['harness_selection']['matched_count'] == 3,
      f"harness_selection.matched_count should be 3, got "
      f"{data['harness_selection']['matched_count']}")
check(data['run_state'] == 'COMPLETE', f"run_state should be COMPLETE, got {data['run_state']!r}")

check(all(h.get('outcome', {}).get('verdict') == 'SUCCESS' for h in harnesses),
      "every harness should report SUCCESS")
check(all(h.get('n_failed') == 0 for h in harnesses),
      "every harness should have 0 failed properties")
check(all(not h.get('failed_properties') for h in harnesses),
      "no harness should list a failed property")

for h in harnesses:
    checks_total = h['checks']['total']
    covers_total = h['covers']['total']
    check(h['n_properties'] == checks_total + covers_total,
          f"{h['name']}: n_properties ({h['n_properties']}) != checks.total "
          f"({checks_total}) + covers.total ({covers_total})")

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)

print("All three harnesses are accounted for and consistent")
EOF
