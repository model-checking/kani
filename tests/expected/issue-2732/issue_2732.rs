// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: -Zmir-opt-level=2

//! Test indexing an array with a constant that is always out of bounds. With `-Zmir-opt-level=2`,
//! GVN turns `C[10]` into a `ConstantIndex` whose `min_length` (11) exceeds the array length (5),
//! after a bounds check that always fails. This used to crash Kani.
//! See https://github.com/model-checking/kani/issues/2732.

const C: [u32; 5] = [0; 5];

#[allow(unconditional_panic)]
fn always_out_of_bounds() -> u32 {
    C[10]
}

/// The reproducer from the issue: the bounds check fails.
#[kani::proof]
fn check_always_out_of_bounds() {
    always_out_of_bounds();
}

/// The same access on a path that is never taken: nothing fails.
#[kani::proof]
fn check_unreachable_out_of_bounds() {
    let taken: bool = kani::any();
    kani::assume(!taken);
    if taken {
        always_out_of_bounds();
    }
}
