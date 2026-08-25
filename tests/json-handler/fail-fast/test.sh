#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test JSON export under --fail-fast: a harness that completed before the abort must
# still appear in the export, and the aborted run's document must still validate.
# The rendered-summary half of this regression lives in
# tests/script-based-pre/fail_fast_keeps_completed/.

set -eu
# The validator's exit status is piped into `tail` below; `pipefail` makes the
# pipeline report it and `set -e` makes that status fatal — both are needed, or a
# failed validation is masked and this test passes regardless.
set -o pipefail

OUTPUT_FILE="fail_fast_output.json"
# Remove the export on every exit path, not just the happy one.
trap 'rm -f "$OUTPUT_FILE"' EXIT

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"

# `sort_harnesses_by_loc` runs later-appearing harnesses first, so sequentially
# `z_passes` completes before `a_fails` aborts the run. The run exits non-zero by
# design (a harness fails), so only this invocation is exempted from `set -e`.
set +e
kani -Z unstable-options test.rs --fail-fast --export-json "$OUTPUT_FILE"
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

# An aborted run still has to produce a structurally valid document.
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


# The completed pass must survive the abort, in the counters and in the results.
summary = data['verification_results']['summary']
for field, want in [('executed', 2), ('successful', 1), ('failed', 1)]:
    check(summary[field] == want,
          f"summary.{field} should be {want}, got {summary[field]}")

results = {r.get('harness_id'): r.get('status')
           for r in data['verification_results']['results']}
check(results.get('z_passes') == 'Success',
      f"z_passes should be Success in results, got {results.get('z_passes')}")
check(results.get('a_fails') == 'Failure',
      f"a_fails should be Failure in results, got {results.get('a_fails')}")

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)

print("The completed harness survives a --fail-fast abort in the export")
EOF
