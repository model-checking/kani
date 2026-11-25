// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

//! A panic while a contract clause is evaluated is a panic of the contract, never permitted by
//! the panic clauses, whatever the order of the attributes.

#[kani::panics_only_if(true)]
#[kani::ensures(|r: &u8| 100 / *r > 0)]
fn post_after(x: u8) -> u8 {
    x
}
#[kani::proof_for_contract(post_after)]
fn check_post_after() {
    post_after(kani::any());
}

#[kani::ensures(|r: &u8| 100 / *r > 0)]
#[kani::panics_only_if(true)]
fn post_before(x: u8) -> u8 {
    x
}
#[kani::proof_for_contract(post_before)]
fn check_post_before() {
    post_before(kani::any());
}

// The panics_if condition panics while panics_only_if(true) is recorded.
#[kani::panics_only_if(true)]
#[kani::panics_if(100 / x > 200)]
fn cond_panics(x: u8) -> u8 {
    x
}
#[kani::proof_for_contract(cond_panics)]
fn check_cond_panics() {
    cond_panics(kani::any());
}

#[kani::panics_if(100 / x > 200)]
#[kani::panics_only_if(true)]
fn cond_panics2(x: u8) -> u8 {
    x
}
#[kani::proof_for_contract(cond_panics2)]
fn check_cond_panics2() {
    cond_panics2(kani::any());
}
