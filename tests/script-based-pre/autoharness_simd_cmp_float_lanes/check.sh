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

kani autoharness -Z autoharness simd_cmp.rs --output-format=terse > out.log 2>&1 || true

echo "--- an instantiation with float mask lanes is skipped, with the reason"
grep -qE '^\| simd_cmp +\| float_only +\| Generic Function: the body calls the SIMD comparison `simd_gt`, which rustc rejects for the result type `f32x4` \(non-integer `f32` lanes\)' out.log

echo "--- a comparison result that is a scalar, not a mask, is skipped"
grep -qE '^\| simd_cmp +\| scalar_result +\| Generic Function: the body calls the SIMD comparison `simd_gt`, which rustc rejects for the result type `i32` \(not a SIMD type\)' out.log

echo "--- an integer mask with the wrong lane count is skipped"
grep -qE '^\| simd_cmp +\| wrong_lane_count +\| Generic Function: the body calls the SIMD comparison `simd_gt`, which rustc rejects for the result type `i64x2` \(2 lanes for a 4-lane comparison\)' out.log

echo "--- where an integer vector also fits, autoharness picks it and verifies"
grep -qE '^\| simd_cmp +\| int_available::<i32x4> .*Success' out.log
absent 'int_available::<f32x4>'

echo "--- an invalid comparison one level down fails its harness, not the run"
grep -q '`simd_gt` with the result type `f32x4` (non-integer `f32` lanes) is not currently supported' out.log
grep -qE '^\| simd_cmp +\| calls_float_only::<f32x4> .*Failure' out.log
absent 'aborting due to'
grep -qE '^Complete - [0-9]+ successfully verified functions, 1 failures' out.log

rm -f out.log
