// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers

//! Test that typed quantifier variables work for non-usize integer types.

#[kani::proof]
#[kani::solver(z3)]
fn check_typed_u64_forall() {
    assert!(kani::forall!(|i: u64 in (0, 10)| i < 10));
}

#[kani::proof]
#[kani::solver(cvc5)]
fn check_typed_u64_exists() {
    assert!(kani::exists!(|i: u64 in (0, 10)| i == 5));
}
