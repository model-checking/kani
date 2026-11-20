#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that JSON export records a failed verification.

set -eu

OUTPUT_FILE="failed_output.json"
# Remove the export on every exit path, not just the happy one: a failing
# validation step exits early under `set -e` and would otherwise leave it behind.
trap 'rm -f "$OUTPUT_FILE"' EXIT

# Find the project root (where scripts/ directory is)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Run Kani with JSON export (expect failure, so don't use -e)
set +e
kani -Z export-json test.rs --export-json "$OUTPUT_FILE"
EXIT_CODE=$?
set -e

# Kani should exit with failure
if [ $EXIT_CODE -eq 0 ]; then
    echo "ERROR: Expected Kani to fail but it succeeded"
    exit 1
fi

echo "Kani failed as expected"

# Check that JSON file was created despite failure
if [ ! -f "$OUTPUT_FILE" ]; then
    echo "ERROR: JSON file $OUTPUT_FILE was not created"
    exit 1
fi

echo "JSON file created despite failure"

python3 << 'EOF'
import json
import sys

with open('failed_output.json', 'r') as f:
    data = json.load(f)

if data['summary']['successful'] != 0:
    print(f"ERROR: Expected 0 successful, got {data['summary']['successful']}")
    sys.exit(1)

if data['summary']['failed'] != 1:
    print(f"ERROR: Expected 1 failed, got {data['summary']['failed']}")
    sys.exit(1)

print("Summary shows correct failure count")

harnesses = data['harnesses']
if len(harnesses) != 1:
    print(f"ERROR: Expected 1 harness, got {len(harnesses)}")
    sys.exit(1)

harness = harnesses[0]
if harness['outcome'].get('verdict') != 'FAILURE':
    print(f"ERROR: Expected outcome.verdict 'FAILURE', got {harness['outcome'].get('verdict')}")
    sys.exit(1)

print("harnesses[0].outcome.verdict is 'FAILURE'")

if harness['n_failed'] < 1:
    print(f"ERROR: n_failed should be >= 1, got {harness['n_failed']}")
    sys.exit(1)

if harness['failure_kind'] == 'NONE':
    print("ERROR: failure_kind should not be NONE on a failing harness")
    sys.exit(1)

print(f"failure_kind field present and non-NONE: {harness['failure_kind']}")

if not harness['failed_properties']:
    print("ERROR: failed_properties should list at least one property")
    sys.exit(1)

print("failed_properties is non-empty")

EOF

echo "All failure validation checks passed!"


