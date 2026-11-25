// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts -Z stubbing

//! When a function is replaced by its contract, the replacement may panic when the
//! `panics_only_if` condition holds and panics when the `panics_if` condition holds. Callers
//! have to account for those panics.

#[kani::panics_only_if(x == 0)]
fn divide(x: u8) -> u8 {
    100 / x
}

#[kani::proof_for_contract(divide)]
fn check_divide() {
    divide(kani::any());
}

fn caller_unguarded(x: u8) -> u8 {
    divide(x)
}

#[kani::proof]
#[kani::stub_verified(divide)]
fn check_caller_unguarded() {
    caller_unguarded(kani::any());
}

fn caller_guarded(x: u8) -> u8 {
    if x == 0 { 0 } else { divide(x) }
}

#[kani::proof]
#[kani::stub_verified(divide)]
fn check_caller_guarded() {
    caller_guarded(kani::any());
}

#[kani::panics_only_if(x == 0)]
fn caller_permits(x: u8) -> u8 {
    divide(x)
}

#[kani::proof_for_contract(caller_permits)]
#[kani::stub_verified(divide)]
fn check_caller_permits() {
    caller_permits(kani::any());
}

#[kani::panics_if(x == 0)]
fn divide_exact(x: u8) -> u8 {
    100 / x
}

#[kani::proof_for_contract(divide_exact)]
fn check_divide_exact() {
    divide_exact(kani::any());
}

/// The replacement of `divide_exact` always panics for zero, so the caller's postcondition holds.
#[kani::panics_only_if(x == 0)]
#[kani::ensures(|_| x != 0)]
fn caller_relies_on_panic(x: u8) -> u8 {
    divide_exact(x)
}

#[kani::proof_for_contract(caller_relies_on_panic)]
#[kani::stub_verified(divide_exact)]
fn check_caller_relies_on_panic() {
    caller_relies_on_panic(kani::any());
}
