// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! 3 proof harnesses, 2 of them are feature-gated.

#[kani::proof]
fn check_proof() {
    assert!(1 == 1);
}

#[cfg(feature = "A")]
#[kani::proof]
fn check_proof_a() {
    assert!(2 == 2);
}

#[cfg(feature = "B")]
#[kani::proof]
fn check_proof_b() {
    assert!(3 == 3);
}
