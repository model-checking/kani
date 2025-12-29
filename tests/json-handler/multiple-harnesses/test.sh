#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test JSON export with multiple harnesses - validates aggregation logic

set -eu

OUTPUT_FILE="multi_harness_output.json"

# Find the project root
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

# Validate JSON structure (suppress verbose output)
python3 "$VALIDATOR" "$OUTPUT_FILE" 2>&1 | tail -1

# Check that JSON contains all 3 harnesses
HARNESS_COUNT=$(python3 -c "import json; data=json.load(open('$OUTPUT_FILE')); print(len(data['harness_metadata']))")

if [ "$HARNESS_COUNT" != "3" ]; then
    echo "ERROR: Expected 3 harnesses in JSON, found $HARNESS_COUNT"
    exit 1
fi

echo "Found 3 harnesses in JSON"

# Clean up
rm -f "$OUTPUT_FILE"

