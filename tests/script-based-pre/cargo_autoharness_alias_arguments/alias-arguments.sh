#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Without --alias-arguments, every reference and pointer argument refers to its own allocation,
# so functions that are only correct for non-aliasing arguments verify successfully; with it, an
# argument may be the same shared reference or raw pointer as an earlier argument of the same
# type, and those functions fail.
# `--bounded-arguments` is passed in both runs so that `bounded_slices_distinct` is not skipped.

# Verify the crate, passing any arguments on to `cargo kani autoharness`, and print:
# - a summary line with the exit status of the command, the number of failed checks, and how many
#   of those are not assertion failures. Every expected failure is the assertion of the function
#   under verification, so a function failing for another reason changes this line;
# - the result row of each function.
# Each line is prefixed with the given label, since the rows of the two runs otherwise differ only
# in the verification result.
run_autoharness() {
    label="$1"
    shift
    # Capture the output first: piping the command straight into `grep` would hide its exit
    # status.
    output=$(cargo kani autoharness -Z autoharness --bounded-arguments --output-format=regular \
        "$@" 2>&1)
    status=$?
    failed_checks=$(echo "${output}" | grep '^Failed Checks:')
    failed=$(echo "${failed_checks}" | grep -c '^Failed Checks:')
    other=$(echo "${failed_checks}" | grep '^Failed Checks:' | grep -vc ': assertion failed: ')
    summary="exit status ${status}, ${failed} failed checks"
    echo "[${label}] ${summary}, ${other} other than assertion failures"
    echo "${output}" | grep -E '^\| cargo_autoharness_alias_arguments \| .*(Success|Failure)' \
        | tr -s ' ' | sort | sed "s/^/[${label}] /"
}

run_autoharness independent
run_autoharness aliasing --alias-arguments
