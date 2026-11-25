// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts -Z unstable-options --prove-safety-only

//! Under `--prove-safety-only`, panics are assumed not to occur, but undefined behavior and
//! postconditions are still checked.

#[kani::panics_only_if(x == 0)]
fn unexpected_panic(x: u8) -> u8 {
    if x == 7 {
        panic!("seven");
    }
    100 / x
}

#[kani::proof_for_contract(unexpected_panic)]
fn check_panics_assumed_away() {
    unexpected_panic(kani::any());
}

#[kani::panics_only_if(x > 200)]
#[kani::ensures(|result: &u8| *result == x)]
fn wrong_postcondition(x: u8) -> u8 {
    if x > 200 {
        panic!("too big");
    }
    x.wrapping_add(1)
}

#[kani::proof_for_contract(wrong_postcondition)]
fn check_postcondition_kept() {
    wrong_postcondition(kani::any());
}
