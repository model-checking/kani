#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test JSON export basic functionality - validates schema_utils.rs functions

set -eu

OUTPUT_FILE="test_output.json"
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

# Validate JSON structure using the validation script (suppress verbose output)
python3 "$VALIDATOR" "$OUTPUT_FILE" 2>&1 | tail -1

# Clean up
rm -f "$OUTPUT_FILE"

