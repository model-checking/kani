// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

//! Checking that we can write contracts for functions returning mutable references.
//! This verifies that Rust correctly identifies contract closures as `FnOnce`.
//! See https://github.com/model-checking/kani/issues/3764

#[kani::requires(*val != 0)]
unsafe fn foo(val: &mut u8) -> &mut u8 {
    val
}

#[kani::proof_for_contract(foo)]
fn harness() {
    let mut x: u8 = kani::any();
    unsafe { foo(&mut x) };
}
