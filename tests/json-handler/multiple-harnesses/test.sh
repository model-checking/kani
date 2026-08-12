#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test JSON export with multiple harnesses - validates aggregation logic

set -eu
# The validator's exit status is piped into `tail` below; without pipefail a failed
# validation is masked by tail's success and this test passes regardless.
set -o pipefail

OUTPUT_FILE="multi_harness_output.json"
# Remove the export on every exit path, not just the happy one: a failing
# validation step exits early under `set -e` and would otherwise leave it behind.
trap 'rm -f "$OUTPUT_FILE"' EXIT

# Find the project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"

# Run Kani with JSON export
kani -Z unstable-options test.rs --export-json "$OUTPUT_FILE"

# Check that JSON file was created
if [ ! -f "$OUTPUT_FILE" ]; then
    echo "ERROR: JSON file $OUTPUT_FILE was not created"
    exit 1
fi

# Validate JSON structure (suppress verbose output)
python3 "$VALIDATOR" "$OUTPUT_FILE" 2>&1 | tail -1

# Check that the export accounts for all three harnesses: not just that the metadata
# lists them, but that the summary counters, the per-harness results, and the
# per-harness detail arrays all agree with each other and with the harnesses that ran.
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


metadata_names = {h.get('pretty_name') for h in data['harness_metadata']}
check(metadata_names == expected,
      f"harness_metadata should list {sorted(expected)}, got {sorted(map(str, metadata_names))}")

summary = data['verification_results']['summary']
for field, want in [('total_harnesses', 3), ('executed', 3),
                    ('successful', 3), ('failed', 0)]:
    check(summary[field] == want,
          f"summary.{field} should be {want}, got {summary[field]}")

results = data['verification_results']['results']
check(len(results) == 3, f"expected 3 results, got {len(results)}")
result_names = {r.get('harness_id') for r in results}
check(result_names == expected,
      f"results should cover {sorted(expected)}, got {sorted(map(str, result_names))}")
check(all(r.get('status') == 'Success' for r in results),
      "every harness should report Success")

# Each per-harness array must cover every harness exactly once. These arrays are built
# in a different order from `results`, so identity has to come from harness_id.
for key in ['error_details', 'property_details', 'cbmc']:
    entries = data[key]
    check(len(entries) == 3, f"expected 3 {key} entries, got {len(entries)}")
    names = {e.get('harness_id') for e in entries}
    check(names == expected,
          f"{key} should cover {sorted(expected)}, got {sorted(map(str, names))}")

check(not any(e.get('has_errors') for e in data['error_details']),
      "no harness should report errors")

for entry in data['property_details']:
    details = entry['property_details']
    check(details.get('failed') == 0,
          f"{entry.get('harness_id')} should have 0 failed properties, "
          f"got {details.get('failed')}")

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)

print("All three harnesses are accounted for and consistent")
EOF


