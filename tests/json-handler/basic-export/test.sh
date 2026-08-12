#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test JSON export basic functionality - validates schema_utils.rs functions

set -eu
# The validator's exit status is piped into `tail` below; without pipefail a failed
# validation is masked by tail's success and this test passes regardless.
set -o pipefail

OUTPUT_FILE="test_output.json"
# Remove the export on every exit path, not just the happy one: a failing
# validation step exits early under `set -e` and would otherwise leave it behind.
trap 'rm -f "$OUTPUT_FILE"' EXIT
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

# Validate JSON structure using the validation script (suppress verbose output)
python3 "$VALIDATOR" "$OUTPUT_FILE" 2>&1 | tail -1


