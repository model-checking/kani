// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Ensure that Kani correctly unwinds the loop with drop instructions.
//! This was related to https://github.com/model-checking/kani/issues/2164

/// Dummy function with a for loop that only runs 2 iterations.
fn bounded_loop<T: Default>(b: bool, other: T) -> T {
    let mut ret = other;
    for i in 0..2 {
        ret = match b {
            true => T::default(),
            false => ret,
        };
    }
    return ret;
}

/// Harness that should succeed. We add a conservative loop bound.
#[kani::proof]
#[kani::unwind(3)]
fn harness() {
    let _ = bounded_loop(kani::any(), ());
}
