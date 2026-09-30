#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that a project without proof harnesses exports run_state NO_HARNESSES_SELECTED.

set -eu
set -o pipefail

OUTPUT_FILE="no_harnesses_output.json"
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

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


check(data['run_state'] == 'NO_HARNESSES_SELECTED',
      f"run_state should be NO_HARNESSES_SELECTED, got {data['run_state']!r}")
check(data['harnesses'] == [], f"harnesses[] should be empty, got {data['harnesses']}")
sel = data['harness_selection']
check(sel['requested_filters'] == [], f"requested_filters should be [], got {sel['requested_filters']}")
check(sel['unmatched_filters'] == [], f"unmatched_filters should be [], got {sel['unmatched_filters']}")
check(sel['matched_count'] == 0, f"matched_count should be 0, got {sel['matched_count']}")
summary = data['summary']
for field in ('total', 'successful', 'failed', 'checks_total', 'checks_success',
              'covers_total', 'covers_satisfied'):
    check(summary[field] == 0, f"summary.{field} should be 0, got {summary[field]}")

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)

print("NO_HARNESSES_SELECTED: empty harnesses[], zero matched_count/summary, on an unfiltered "
      "project with no selectable proof harness")
EOF
