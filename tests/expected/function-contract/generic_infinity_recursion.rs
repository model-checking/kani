// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Check Kani handling of generics and recursion with function contracts.

#[kani::requires(x != 0)]
#[kani::recursion]
fn foo<T: std::cmp::PartialEq<i32>>(x: T) {
    assert_ne!(x, 0);
    foo(x);
}

#[kani::proof_for_contract(foo)]
fn foo_harness() {
    let input: i32 = kani::any();
    foo(input);
}
