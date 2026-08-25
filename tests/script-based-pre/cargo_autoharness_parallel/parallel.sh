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
if ! parallel_output=$(RAYON_NUM_THREADS=2 cargo kani autoharness -Z autoharness 2>&1); then
    echo "$parallel_output"
    exit 1
fi
if ! serial_output=$(RAYON_NUM_THREADS=2 cargo kani autoharness -Z autoharness --jobs=1 2>&1); then
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
if echo "$parallel_output" | grep -qE '^Check [0-9]+:'; then
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
