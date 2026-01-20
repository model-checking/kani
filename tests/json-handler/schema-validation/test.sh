#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test JSON schema structure from schema_utils.rs functions
# This test uses the schema template to validate the JSON output

set -eu

OUTPUT_FILE="schema_output.json"

# Find the project root (where scripts/ directory is)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"

# Run Kani with JSON export
kani test.rs --export-json "$OUTPUT_FILE"

# Check that JSON file was created
if [ ! -f "$OUTPUT_FILE" ]; then
    echo "ERROR: JSON file $OUTPUT_FILE was not created"
    exit 1
fi

echo "JSON file created"

# Validate basic structure using schema template
python3 "$VALIDATOR" "$OUTPUT_FILE"

# Validate each field from schema template dynamically
echo ""
echo "Validating individual fields from schema:"

# Get all top-level keys from schema and validate each that exists in JSON
SCHEMA_FILE="$(dirname "$0")/kani_json_schema.json"
python3 << EOF
import json
import subprocess
import sys

with open('$SCHEMA_FILE', 'r') as f:
    schema = json.load(f)

with open('$OUTPUT_FILE', 'r') as f:
    data = json.load(f)

all_passed = True
for key in schema.keys():
    if key not in data:
        continue
    
    result = subprocess.run(
        ['python3', '$VALIDATOR', '$OUTPUT_FILE', '--field-path', key],
        capture_output=True
    )
    if result.returncode == 0:
        print(f"  {key}: PASSED")
    else:
        print(f"  {key}: FAILED")
        all_passed = False

if not all_passed:
    sys.exit(1)
EOF

if [ $? -ne 0 ]; then
    echo "Field validation failed"
    exit 1
fi

# Clean up
rm -f "$OUTPUT_FILE"

echo ""
echo "All validations passed"

