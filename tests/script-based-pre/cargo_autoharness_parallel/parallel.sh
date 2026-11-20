#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Autoharness defaults to parallel verification (-j) with terse output. Harness results
# arrive in nondeterministic order, so assert on order-independent evidence: the absence of
# per-check detail, the thread prefixes, the (sorted) per-function summary lines, and the
# totals line. Then compare those structured summaries with an explicit --jobs=1 run.
#
# RAYON_NUM_THREADS pins the default pool size, so the thread prefixes below appear regardless
# of how much parallelism the machine (or its cgroup/CPU affinity) actually offers. Rayon only
# consults the environment variable when the thread count is left unset; --jobs=1 must override
# it and preserve the serial opt-out.
parallel_output=$(RAYON_NUM_THREADS=2 cargo kani autoharness -Z autoharness -Z unstable-options --output-into-files --target-dir target 2>&1)
# f5 intentionally fails verification, so both runs must exit with status 1.
if [[ $? -ne 1 ]]; then
    echo "$parallel_output"
    exit 1
fi

# Each file already identifies its harness. Keep result lines usable by tools
# that match the beginning of the line, just as in a sequential run.
file_results=$(grep -h '^VERIFICATION:- ' target/result_output_dir/*)
if [[ $(echo "$file_results" | grep -c '^VERIFICATION:- SUCCESSFUL$') -eq 4 ]] &&
    [[ $(echo "$file_results" | grep -c '^VERIFICATION:- FAILED$') -eq 1 ]] &&
    ! grep -q '^Thread [0-9]*:' target/result_output_dir/*; then
    echo "FILES: yes"
else
    echo "FILES: no"
    exit 1
fi

serial_output=$(RAYON_NUM_THREADS=2 cargo kani autoharness -Z autoharness --jobs=1 2>&1)
if [[ $? -ne 1 ]]; then
    echo "$serial_output"
    exit 1
fi

normalized_results() {
    local output=$1
    {
        echo "$output" | grep -oE '\| f[0-9] .*(Success|Failure)' | tr -s ' ' | sort
        echo "$output" | grep "^Complete - "
    }
}

# Terse output omits the per-check detail that `--output-format=regular` prints.
if echo "$parallel_output" | grep -qE '^(Thread [0-9]+: )?Check [0-9]+:'; then
    echo "TERSE: no"
    echo "$parallel_output"
else
    echo "TERSE: yes"
fi

# Harness results are prefixed with the thread that produced them when the pool has more than
# one thread.
if echo "$parallel_output" | grep -q "Thread [0-9]*:"; then
    echo "PARALLEL: yes"
else
    echo "PARALLEL: no"
    echo "$parallel_output"
fi

# Regression for #4438: every nonempty line in the parallel harness output must identify
# its thread, including the verification result and timing, not just the first line.
if echo "$parallel_output" | awk '
    expect_separator { if (NF) missing_separator = 1; expect_separator = 0 }
    /^Thread [0-9]+:/ { in_harness_output = 1 }
    /^Manual Harness Summary:/ { in_harness_output = 0 }
    in_harness_output && NF && $0 !~ /^Thread [0-9]+:/ { missing_prefix = 1 }
    /^Thread [0-9]+: VERIFICATION:- SUCCESSFUL$/ { successes++ }
    /^Thread [0-9]+: VERIFICATION:- FAILED$/ { failures++ }
    /^Thread [0-9]+: Failed Checks:/ { failed_checks++ }
    /^Thread [0-9]+:  File:/ { locations++ }
    /^Thread [0-9]+: Verification Time:/ { expect_separator = 1 }
    END { exit (missing_prefix || missing_separator || successes != 4 ||
                failures != 1 || failed_checks != 1 || locations != 1) }
'; then
    echo "PREFIXED RESULTS: yes"
else
    echo "PREFIXED RESULTS: no"
    echo "$parallel_output"
fi

if echo "$serial_output" | grep -q "Thread [0-9]*:"; then
    echo "SERIAL: no"
    echo "$serial_output"
else
    echo "SERIAL: yes"
fi

parallel_results=$(normalized_results "$parallel_output")
serial_results=$(normalized_results "$serial_output")
if [[ "$parallel_results" == "$serial_results" ]]; then
    echo "EQUIVALENT: yes"
else
    echo "EQUIVALENT: no"
    diff <(echo "$serial_results") <(echo "$parallel_results")
fi

echo "$parallel_results"
