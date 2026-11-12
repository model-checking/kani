#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Run `autoharness` with different unwinding options and print one `[<run>] result:` line per
# harness, so that the expected file can tell the runs apart.
set -u

run() {
    local label=$1
    shift
    local output status
    output=$(cargo kani autoharness -Z autoharness --output-format=regular "$@" 2>&1)
    status=$?
    echo "$output"
    echo "$output" | awk -v label="$label" '
        { sub(/^Thread [0-9]+: /, "") }
        /^Checking harness /{ name = $3; sub(/\.\.\.$/, "", name) }
        /^Autoharness: Checking function / { name = $4 }
        /^VERIFICATION:- / { print "[" label "] result: " name " " $2 }
    '
    echo "[$label] exit status: $status"
}

# The autoharness defaults: the automatic harnesses get 20 and the manual harnesses get nothing
# beyond their own `#[kani::unwind]`. The include pattern mirrors the reproduction in #5103, where
# the pattern only selects functions unrelated to the manual harness.
echo "[default]"
run default --include-pattern 'loop_'

# An explicit `--default-unwind` applies to every harness: it is below the autoharness default,
# so `loop_10_iterations` now fails too, and so does `manual_loop_30`. `#[kani::unwind]` still wins.
echo "[default-unwind 10]"
run default-unwind-10 --default-unwind 10

# An explicit `--default-unwind` above the autoharness default lets `loop_30_iterations` pass.
echo "[default-unwind 40]"
run default-unwind-40 --default-unwind 40
