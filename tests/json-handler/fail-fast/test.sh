#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that a harness completed before a --fail-fast abort stays in the export.

set -eu
set -o pipefail

OUTPUT_FILE="fail_fast_output.json"
COMPLETE_OUTPUT_FILE="fail_fast_complete_output.json"
trap 'rm -f "$OUTPUT_FILE" "$COMPLETE_OUTPUT_FILE"' EXIT

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"

set +e
kani -Z export-json test.rs --fail-fast --export-json "$OUTPUT_FILE"
CODE=$?
set -e

# The run must fail overall; a --fail-fast run that exits 0 is a different bug.
if [ "$CODE" -eq 0 ]; then
    echo "ERROR: --fail-fast run exited 0 despite a failing harness"
    exit 1
fi

if [ ! -f "$OUTPUT_FILE" ]; then
    echo "ERROR: JSON file $OUTPUT_FILE was not created"
    exit 1
fi

python3 "$VALIDATOR" "$OUTPUT_FILE" 2>&1 | tail -1

set +e
kani -Z export-json test.rs --fail-fast --harness z_passes --harness a_fails \
    --export-json "$COMPLETE_OUTPUT_FILE"
COMPLETE_CODE=$?
set -e

if [ "$COMPLETE_CODE" -eq 0 ]; then
    echo "ERROR: the COMPLETE --fail-fast run exited 0 despite a failing harness"
    exit 1
fi

if [ ! -f "$COMPLETE_OUTPUT_FILE" ]; then
    echo "ERROR: JSON file $COMPLETE_OUTPUT_FILE was not created"
    exit 1
fi

python3 "$VALIDATOR" "$COMPLETE_OUTPUT_FILE" 2>&1 | tail -1

python3 << EOF
import json
import sys

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


def verdicts_of(data):
    return {h['name']: h['outcome'].get('verdict') for h in data['harnesses']}


with open('$COMPLETE_OUTPUT_FILE', 'r') as f:
    data = json.load(f)
check(verdicts_of(data) == {'z_passes': 'SUCCESS', 'a_fails': 'FAILURE'},
      f"harness verdicts should be exactly z_passes=SUCCESS and a_fails=FAILURE, got "
      f"{verdicts_of(data)}")
check(data['run_state'] == 'COMPLETE',
      f"run_state should be COMPLETE when the abort lands on the last selected harness, "
      f"got {data['run_state']!r}")
check(data['harness_selection']['matched_count'] == 2,
      f"harness_selection.matched_count should be 2, got "
      f"{data['harness_selection']['matched_count']}")
check(data['summary']['total'] == 2,
      f"summary.total should be 2, got {data['summary']['total']}")
check(data['harness_selection']['matched_count'] == data['summary']['total'],
      "matched_count should equal summary.total when nothing is skipped")

with open('$OUTPUT_FILE', 'r') as f:
    data = json.load(f)
summary = data['summary']
for field, want in [('total', 2), ('successful', 1), ('failed', 1)]:
    check(summary[field] == want,
          f"summary.{field} should be {want}, got {summary[field]}")
check(data['run_state'] == 'PARTIAL',
      f"run_state should be PARTIAL: a0_never_runs is queued behind the fail-fast abort and "
      f"never produces an entry, got {data['run_state']!r}")
check(data['harness_selection']['matched_count'] == 3,
      f"harness_selection.matched_count should be 3, got "
      f"{data['harness_selection']['matched_count']}")
check(summary['total'] < data['harness_selection']['matched_count'],
      f"summary.total ({summary['total']}) should be < matched_count "
      f"({data['harness_selection']['matched_count']}) under PARTIAL")
check(verdicts_of(data) == {'z_passes': 'SUCCESS', 'a_fails': 'FAILURE'},
      f"harnesses[] should hold exactly z_passes=SUCCESS and a_fails=FAILURE (no entry for "
      f"a0_never_runs), got {verdicts_of(data)}")

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)

print("A --fail-fast abort keeps the completed harnesses in the export: PARTIAL when later "
      "harnesses were skipped, COMPLETE when the abort lands on the last selected harness")
EOF
