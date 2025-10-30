#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test JSON export with failed verification - validates error capture

set -eu

OUTPUT_FILE="failed_output.json"

# Find the project root (where scripts/ directory is)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Run Kani with JSON export (expect failure, so don't use -e)
set +e
kani test.rs --export-json "$OUTPUT_FILE"
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

# Validate that JSON contains failure information
python3 << 'EOF'
import json
import sys

with open('failed_output.json', 'r') as f:
    data = json.load(f)

# Check verification_results shows failure
vr = data['verification_results']
summary = vr['summary']

if summary['successful'] != 0:
    print(f"ERROR: Expected 0 successful, got {summary['successful']}")
    sys.exit(1)

if summary['failed'] != 1:
    print(f"ERROR: Expected 1 failed, got {summary['failed']}")
    sys.exit(1)

print("Summary shows correct failure count")

# Check that results array contains failure status
results = vr['results']
if len(results) != 1:
    print(f"ERROR: Expected 1 result, got {len(results)}")
    sys.exit(1)

if results[0]['status'] != 'Failure':
    print(f"ERROR: Expected status 'Failure', got {results[0]['status']}")
    sys.exit(1)

print("Result status is 'Failure'")

# Check that error_details exists and has_errors is true
if 'error_details' not in data:
    print("ERROR: error_details field missing")
    sys.exit(1)

# error_details is an object (single harness)
error_details = data['error_details']
if not error_details.get('has_errors'):
    print("ERROR: has_errors should be true")
    sys.exit(1)

print("error_details.has_errors is true")

# Verify error_type is present
if 'error_type' not in error_details:
    print("ERROR: error_type field missing")
    sys.exit(1)

print("error_type field present")

EOF

echo "All failure validation checks passed!"

# Clean up
rm -f "$OUTPUT_FILE"

