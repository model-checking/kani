// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

//! `should_panic` needs a failed panic check: a panic that `panics_only_if` permits is not a
//! failure, so it does not count, while a panic it does not permit does.

#[kani::panics_only_if(x == 0)]
fn divide(x: u8) -> u8 {
    100 / x
}

#[kani::proof_for_contract(divide)]
#[kani::should_panic]
fn check_permitted_panic() {
    divide(kani::any());
}

#[kani::panics_only_if(x == 0)]
fn divide_or_panic(x: u8) -> u8 {
    if x == 1 {
        panic!("one");
    }
    100 / x
}

#[kani::proof_for_contract(divide_or_panic)]
#[kani::should_panic]
fn check_unpermitted_panic() {
    divide_or_panic(kani::any());
}
