// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that automatic harnesses terminate on functions with loops.

// Since foo()'s arguments implement Arbitrary, we will attempt to verify it,
// and enter an infinite loop.
// Unclear what the best solution to this problem is; perhaps this is just a known limitation
// and the user needs to specify some command line flag to skip this function,
// or we can conservatively skip functions with loops that don't have loop contracts.
fn infinite_loop() {
    loop {}
}

/// Euclid's algorithm for calculating the GCD of two numbers
#[kani::requires(x != 0 && y != 0)]
#[kani::ensures(|result : &u8| *result != 0 && x % *result == 0 && y % *result == 0)]
fn gcd(mut x: u8, mut y: u8) -> u8 {
    (x, y) = (if x > y { x } else { y }, if x > y { y } else { x });
    loop {
        let res = x % y;
        if res == 0 {
            return y;
        }

        x = y;
        y = res;
    }
}

// Since we can specify an unwinding bound in a manual harness,
// the proof will terminate.
// Automatic harnesses, however, do not support unwinding bounds,
// so the proof does not terminate.
#[kani::proof_for_contract(gcd)]
#[kani::unwind(12)]
fn gcd_harness() {
    gcd(kani::any(), kani::any());
}
