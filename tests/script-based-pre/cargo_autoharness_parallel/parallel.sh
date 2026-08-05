#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Autoharness defaults to parallel verification (-j) with terse output. Harness results
# arrive in nondeterministic order, so assert on order-independent evidence: the absence of
# per-check detail, the thread prefixes, the (sorted) per-function summary lines, and the
# totals line.
#
# RAYON_NUM_THREADS pins the pool size, so the thread prefixes below appear regardless of how
# much parallelism the machine (or its cgroup/CPU affinity) actually offers. This still checks
# the default: rayon only consults the environment variable when the thread count is left
# unset, which is exactly what the defaulted `--jobs` does -- sequential verification passes an
# explicit `num_threads(1)` that overrides it.
output=$(RAYON_NUM_THREADS=2 cargo kani autoharness -Z autoharness 2>&1)

# Terse output omits the per-check detail that `--output-format=regular` prints.
if echo "$output" | grep -qE '^Check [0-9]+:'; then
    echo "TERSE: no"
    echo "$output"
else
    echo "TERSE: yes"
fi

# Harness results are prefixed with the thread that produced them when the pool has more than
# one thread.
if echo "$output" | grep -q "Thread [0-9]*:"; then
    echo "PARALLEL: yes"
else
    echo "PARALLEL: no"
    echo "$output"
fi

echo "$output" | grep -oE '\| f[0-9] .*(Success|Failure)' | tr -s ' ' | sort
echo "$output" | grep "^Complete - "
