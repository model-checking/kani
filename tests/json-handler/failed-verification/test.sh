#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test JSON export with failed verification - validates error capture

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
kani -Z unstable-options test.rs --export-json "$OUTPUT_FILE"
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

# error_details is an array with one entry per harness, each identified by harness_id
error_details = data['error_details']
if not isinstance(error_details, list):
    print(f"ERROR: error_details should be a list, got {type(error_details).__name__}")
    sys.exit(1)

if len(error_details) != 1:
    print(f"ERROR: Expected 1 error_details entry, got {len(error_details)}")
    sys.exit(1)

entry = error_details[0]

if 'harness_id' not in entry:
    print("ERROR: harness_id field missing")
    sys.exit(1)

if not entry.get('has_errors'):
    print("ERROR: has_errors should be true")
    sys.exit(1)

print("error_details.has_errors is true")

# Verify error_type is present
if 'error_type' not in entry:
    print("ERROR: error_type field missing")
    sys.exit(1)

print("error_type field present")

EOF

echo "All failure validation checks passed!"


