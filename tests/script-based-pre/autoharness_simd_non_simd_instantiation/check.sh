#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# A negated `grep` is exempt from `errexit`, so spell out the absent-pattern check.
absent() {
    if grep -qE "$1" out.log; then
        echo "unexpectedly found '$1'"
        exit 1
    fi
}

kani autoharness -Z autoharness simd_generic.rs --output-format=terse > out.log 2>&1 || true

echo "--- the reproducer is instantiated with the SIMD type, not with a scalar"
grep -qE '^\| simd_generic +\| imin::<I32x2>' out.log
absent '^\| simd_generic +\| imin::<i32>'

echo "--- reaching the invalid instantiation through a call reports it, rather than crashing"
grep -q '`simd_lt` on non-SIMD type `i32` is not currently supported' out.log
grep -q 'in imin::<i32>' out.log
grep -qE '^\| simd_generic +\| calls_imin::<i32> .*Failure' out.log

echo "--- a function whose SIMD operand types are fixed is unaffected"
grep -qE '^\| simd_generic +\| compare_vectors .*Success' out.log

echo "--- and the run finishes, rather than aborting the compiler"
grep -q 'Complete - 3 successfully verified functions, 1 failures, 4 total' out.log

rm -f out.log
