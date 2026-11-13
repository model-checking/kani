// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Input for `test_scripts.py`. Not compiled; each harness exercises one rule.

// kani::assume(false) inside a comment must be ignored.
#[kani::requires(n > 10 && n < 3)]
fn contract_vacuous(n: u32) -> u32 {
    n
}

#[kani::requires(n > 0)]
fn contract_ok(n: u32) -> u32 {
    n
}

#[kani::proof]
fn clean() {
    let x: u8 = kani::any();
    let s = "kani::assume(false)";
    kani::assume(x < 100);
    assert!(x as u32 + 1 > 0);
    kani::cover!(true);
}

#[kani::proof]
fn contradictory() {
    let x: u8 = kani::any();
    kani::assume(x > 10);
    kani::assume(5 > x);
}

#[kani::proof]
fn assume_false() {
    kani::assume(false);
}

#[cfg_attr(kani, kani::proof)]
#[kani::unwind(3)]
fn pinned_and_late() {
    let x: u8 = kani::any();
    let r: Option<u8> = kani::any();
    kani::assume(r.is_some());
    for i in 0..2 {
        assert!(i < 2);
    }
    kani::assume(x == 7);
}

#[kani::proof]
fn casts_are_not_variables() {
    let x: u8 = kani::any();
    let y: u8 = kani::any();
    kani::assume(x as u32 > 10);
    kani::assume(y as u32 < 5);
}

#[kani::proof]
fn no_inputs() {
    let v = [1, 2, 3];
    while false {}
    helper(v.len(), 'a', '}');
}

#[kani::proof_for_contract(contract_ok)]
#[kani::stub(a::b, c::d)]
fn contract_harness() {
    contract_ok(kani::any());
}
