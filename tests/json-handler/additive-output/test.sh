#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that --export-json leaves stdout, --sarif and --output-into-files output unchanged, and that `-` names a file.

set -eu
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

# Runtime and verification time lines differ between runs; everything else must match.
mask_volatile() {
    sed -E 's/^(Runtime [A-Za-z][A-Za-z -]*|Verification Time): [0-9.eE+-]+s$/\1: <MASKED>s/'
}

check_exit() {
    local label="$1" want_exit="$2" base_code="$3" export_code="$4"
    if [ "$export_code" -ne "$base_code" ] \
        || { [ "$want_exit" = zero ] && [ "$base_code" -ne 0 ]; } \
        || { [ "$want_exit" = nonzero ] && [ "$base_code" -eq 0 ]; }; then
        echo "ERROR: $label exit statuses: baseline $base_code, with --export-json $export_code (want equal, $want_exit)"
        exit 1
    fi
}

check_verdict() {
    python3 - "$1" "$2" "$3" << 'PYEOF'
import json
import sys

path, harness, want = sys.argv[1:]
with open(path) as f:
    hs = json.load(f)["harnesses"]
if [h["name"] for h in hs] != [harness]:
    print(f"ERROR: expected only {harness} in {path}, got {[h['name'] for h in hs]}")
    sys.exit(1)
verdict = hs[0]["outcome"].get("verdict")
if verdict != want:
    print(f"ERROR: {harness} verdict in {path} should be {want}, got {verdict}")
    sys.exit(1)
PYEOF
}

check_additive() {
    local harness="$1" want_exit="$2" want_verdict="$3"
    local dir="$WORK_DIR/$harness"
    mkdir "$dir"
    cp "$SCRIPT_DIR/test.rs" "$dir/test.rs"
    (
        cd "$dir"
        local base_code=0 export_code=0
        kani --sarif base.sarif test.rs --harness "$harness" --exact > base_stdout.txt || base_code=$?
        kani -Z export-json --sarif export.sarif test.rs --harness "$harness" --exact \
            --export-json out.json > export_stdout.txt || export_code=$?
        check_exit "$harness" "$want_exit" "$base_code" "$export_code"
        if [ ! -f out.json ]; then
            echo "ERROR: JSON file out.json was not created for $harness"
            exit 1
        fi
        python3 "$VALIDATOR" out.json 2>&1 | tail -1
        check_verdict out.json "$harness" "$want_verdict"
        mask_volatile < base_stdout.txt > base_stdout.masked.txt
        mask_volatile < export_stdout.txt > export_stdout.masked.txt
        if ! diff -u base_stdout.masked.txt export_stdout.masked.txt; then
            echo "ERROR: --export-json changed rendered stdout for $harness"
            exit 1
        fi
        if ! diff -u base.sarif export.sarif; then
            echo "ERROR: --export-json changed --sarif output for $harness"
            exit 1
        fi
        python3 - base.sarif "$want_exit" << 'PYEOF'
import json
import sys

path, want_exit = sys.argv[1:]
with open(path) as f:
    results = json.load(f)["runs"][0]["results"]
if bool(results) != (want_exit == "nonzero"):
    print(f"ERROR: {path} results[] is {'non-empty' if results else 'empty'} but the exit status "
          f"is expected to be {want_exit}")
    sys.exit(1)
PYEOF

        base_code=0
        export_code=0
        kani -Z unstable-options --output-into-files test.rs --harness "$harness" --exact || base_code=$?
        mv "result_output_dir/$harness" base_per_harness.txt
        rm -rf result_output_dir
        kani -Z unstable-options -Z export-json --output-into-files test.rs --harness "$harness" \
            --exact --export-json files_out.json || export_code=$?
        check_exit "$harness under --output-into-files" "$want_exit" "$base_code" "$export_code"
        if [ ! -f files_out.json ]; then
            echo "ERROR: --export-json output missing for $harness under --output-into-files"
            exit 1
        fi
        python3 "$VALIDATOR" files_out.json 2>&1 | tail -1
        check_verdict files_out.json "$harness" "$want_verdict"
        if [ ! -f "result_output_dir/$harness" ] || ! grep -q "VERIFICATION" "result_output_dir/$harness"; then
            echo "ERROR: --output-into-files produced no usable per-harness file for $harness"
            exit 1
        fi
        mask_volatile < base_per_harness.txt > base_per_harness.masked.txt
        mask_volatile < "result_output_dir/$harness" > export_per_harness.masked.txt
        if ! diff -u base_per_harness.masked.txt export_per_harness.masked.txt; then
            echo "ERROR: --export-json changed the --output-into-files per-harness file for $harness"
            exit 1
        fi
    )
}

check_additive verify_add_numbers zero SUCCESS
check_additive verify_add_numbers_fails nonzero FAILURE

mkdir "$WORK_DIR/dash"
cp "$SCRIPT_DIR/test.rs" "$WORK_DIR/dash/test.rs"
(
    cd "$WORK_DIR/dash"
    kani -Z export-json test.rs --harness verify_add_numbers --exact --export-json - > dash_stdout.txt
)

if [ ! -f "$WORK_DIR/dash/-" ]; then
    echo "ERROR: no literal file named '-' was created"
    exit 1
fi
python3 "$VALIDATOR" "$WORK_DIR/dash/-" 2>&1 | tail -1

if grep -q '"schema_version"' "$WORK_DIR/dash/dash_stdout.txt"; then
    echo "ERROR: the JSON document leaked onto stdout"
    exit 1
fi
if python3 "$VALIDATOR" "$WORK_DIR/dash/dash_stdout.txt" > /dev/null 2>&1; then
    echo "ERROR: stdout itself validates as the JSON export"
    exit 1
fi

echo "additive-output: stdout (apart from timing lines), --sarif and --output-into-files are unaffected by --export-json, and '-' is a literal filename"
