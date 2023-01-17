// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that all other checks are reported as `UNDETERMINED` when there is an
//! unwinding assertion failure in a recursive program.

fn factorial(x: u32) -> u32 {
    if x == 1 { x } else { x * factorial(x - 1) }
}

#[kani::proof]
#[kani::unwind(3)]
fn main() {
    let x = 5;
    let f = factorial(x);
    assert_eq!(f, 120);
}
