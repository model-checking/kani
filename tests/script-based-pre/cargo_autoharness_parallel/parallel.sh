#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Autoharness defaults to parallel verification (-j) with terse output. Harness results
# arrive in nondeterministic order, so assert on order-independent evidence: the thread
# prefixes, the (sorted) per-function summary lines, and the totals line.
output=$(cargo kani autoharness -Z autoharness 2>&1)

if echo "$output" | grep -q "Thread [0-9]*:"; then
    echo "PARALLEL: yes"
else
    echo "PARALLEL: no"
    echo "$output"
fi

echo "$output" | grep -oE '\| f[0-9] .*(Success|Failure)' | tr -s ' ' | sort
echo "$output" | grep "^Complete - "
