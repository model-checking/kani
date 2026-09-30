#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that rejected arguments fail before compilation and leave an existing export file or directory untouched.

set -eu

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURE="$SCRIPT_DIR/fixture.rs"

OUT="out.json"
DIR_OUT="out_dir"
COMPILE_ERROR='error\[E[0-9]+\]'
SAVED="saved_sentinel.txt"
cleanup() { rm -f "$OUT" "$SAVED"; rm -rf "$DIR_OUT"; }
trap cleanup EXIT

fail=0

check_rejected() {
    local label="$1"
    local expected_diagnostic="$2"
    local sentinel="$3"
    shift 3
    mkdir -p "$(dirname "$sentinel")"
    printf 'sentinel: not a real export document\n' > "$sentinel"
    cp "$sentinel" "$SAVED"
    local out code
    set +e
    out=$(kani "$@" 2>&1)
    code=$?
    set -e
    if [ "$code" -eq 0 ]; then
        echo "FAIL [$label]: exited 0, expected rejection before compilation"
        echo "$out"
        fail=1
        return
    fi
    if ! grep -qF "$expected_diagnostic" <<< "$out"; then
        echo "FAIL [$label]: expected diagnostic containing '$expected_diagnostic', got:"
        echo "$out"
        fail=1
        return
    fi
    if grep -q 'VERIFICATION' <<< "$out" || grep -qE "$COMPILE_ERROR" <<< "$out"; then
        echo "FAIL [$label]: compilation or verification ran despite the rejection"
        echo "$out"
        fail=1
        return
    fi
    if [ ! -f "$sentinel" ]; then
        echo "FAIL [$label]: the pre-existing file $sentinel was deleted"
        fail=1
        return
    fi
    if ! cmp -s "$sentinel" "$SAVED"; then
        echo "FAIL [$label]: the pre-existing file $sentinel was modified"
        fail=1
        return
    fi
    echo "OK [$label]: rejected before compilation, $sentinel unchanged"
}

set +e
control=$(kani "$FIXTURE" 2>&1)
control_code=$?
set -e
if [ "$control_code" -eq 0 ] || ! grep -qE "$COMPILE_ERROR" <<< "$control"; then
    echo "FAIL: the fixture should fail to compile, so a compilation attempt is observable"
    echo "$control"
    exit 1
fi

check_rejected "no gate at all" \
    "The \`--export-json\` option is unstable and requires \`-Z export-json\` to be used." \
    "$OUT" "$FIXTURE" --export-json "$OUT"
check_rejected "only -Z unstable-options (not -Z export-json)" \
    "The \`--export-json\` option is unstable and requires \`-Z export-json\` to be used." \
    "$OUT" -Z unstable-options "$FIXTURE" --export-json "$OUT"
check_rejected "--output-format=old" \
    "Conflicting options: --export-json isn't compatible with --output-format=old." \
    "$OUT" -Z export-json "$FIXTURE" --export-json "$OUT" --output-format=old
check_rejected "--only-codegen" \
    "Conflicting options: --export-json isn't compatible with --only-codegen." \
    "$OUT" -Z export-json "$FIXTURE" --export-json "$OUT" --only-codegen

check_rejected "existing directory" \
    "\`--export-json\` argument \`$DIR_OUT\` is a directory" \
    "$DIR_OUT/sentinel.txt" -Z export-json "$FIXTURE" --export-json "$DIR_OUT"

if [ "$fail" -ne 0 ]; then
    exit 1
fi

echo "All gate-rejections checks passed!"
