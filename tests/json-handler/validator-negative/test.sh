#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Negative test for `scripts/validate_json_export.py` itself, not for `--export-json`'s output.
# It does not invoke Kani or CBMC at all: it only exercises the validator against synthetic
# fixtures built from the schema template.
#
# Before the validator hardening (targeting #4472's follow-up), `validate_structure_recursive`
# checked keys and nesting but never leaf value types, and count semantics were never checked at
# all. That let a document like `"failed": -100` or `"successful": "yes"` validate as OK, giving
# false CI confidence to anyone using this validator as a pass/fail oracle. This test asserts the
# opposite is now true: a document that is structurally well-formed but carries a corrupted value
# is REJECTED (non-zero exit).

set -eu
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"
SCHEMA="$PROJECT_ROOT/tests/json-handler/schema-validation/kani_json_schema.json"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

python3 "$SCRIPT_DIR/build_fixtures.py" "$SCHEMA" "$WORK_DIR"

echo "Checking a well-formed export still validates..."
python3 "$VALIDATOR" "$WORK_DIR/valid_export.json"

echo ""
echo "Checking a malformed export (verification_results.summary.failed: -1) is now rejected..."
if python3 "$VALIDATOR" "$WORK_DIR/malformed_negative_count.json"; then
    echo "ERROR: validator accepted a document with a negative 'failed' count"
    exit 1
fi

echo ""
echo "Checking a malformed export (verification_results.summary.successful: \"yes\") is now rejected..."
if python3 "$VALIDATOR" "$WORK_DIR/malformed_wrong_type.json"; then
    echo "ERROR: validator accepted a document with successful:\"yes\" (a string, not a number)"
    exit 1
fi

echo ""
echo "Checking a malformed export (summary.executed disagreeing with len(results)) is now rejected..."
if python3 "$VALIDATOR" "$WORK_DIR/malformed_count_mismatch.json"; then
    echo "ERROR: validator accepted a document where summary.executed disagreed with len(results)"
    exit 1
fi

echo ""
echo "Checking a malformed export (run_state: \"complete\" with results: []) is now rejected..."
if python3 "$VALIDATOR" "$WORK_DIR/malformed_complete_with_empty_results.json"; then
    echo "ERROR: validator accepted run_state:\"complete\" with an empty results array"
    exit 1
fi

echo ""
echo "Checking a degraded export (timed-out harness, property counts: null) still validates..."
python3 "$VALIDATOR" "$WORK_DIR/nullable_timeout.json"

echo ""
echo "Checking a degraded export (--smt2 run, no solver/object_bits) still validates..."
python3 "$VALIDATOR" "$WORK_DIR/nullable_smt2.json"

echo ""
echo "Checking a degraded export (partial cbmc_stats) still validates..."
python3 "$VALIDATOR" "$WORK_DIR/nullable_partial_stats.json"

echo ""
echo "Checking a degraded export (failed tool/CBMC version probes) still validates..."
python3 "$VALIDATOR" "$WORK_DIR/nullable_missing_versions.json"

echo ""
echo "Checking a malformed export (verification_results.summary.executed: null) is rejected..."
if python3 "$VALIDATOR" "$WORK_DIR/null_in_required_field.json"; then
    echo "ERROR: validator accepted a null in a non-nullable required field"
    exit 1
fi

echo ""
echo "All validator negative-test checks passed"
