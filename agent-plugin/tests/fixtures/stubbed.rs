// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Single-file fixture for the agent plugin smoke tests: a harness that fails only because of
//! a wrong stub. Concrete playback replays the real code, which passes.

fn double(x: u8) -> u8 {
    x.wrapping_mul(2)
}

#[cfg(kani)]
fn bad_double_model(_x: u8) -> u8 {
    1
}

#[cfg(kani)]
#[kani::proof]
#[kani::stub(double, bad_double_model)]
fn check_double_is_even() {
    let x: u8 = kani::any();
    assert!(double(x) % 2 == 0);
}

#[cfg(kani)]
fn nondet_double_model(_x: u8) -> u8 {
    kani::any()
}

/// The stub consumes a nondeterministic value that the real code does not: its playback test
/// ends with concrete values left over.
#[cfg(kani)]
#[kani::proof]
#[kani::stub(double, nondet_double_model)]
fn check_double_is_even_nondet_stub() {
    let x: u8 = kani::any();
    assert!(double(x) % 2 == 0);
}
