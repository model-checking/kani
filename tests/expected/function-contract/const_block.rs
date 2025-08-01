// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

//! Checking that constant blocks are correctly verified in contracts.
//! See https://github.com/model-checking/kani/issues/3905

#[derive(PartialEq)]
enum Enum {
    First,
    Second,
}

#[kani::ensures(|result| *result == Enum::First)]
const fn first() -> Enum {
    const { Enum::First }
}

#[kani::ensures(|result| *result == Enum::Second)]
const fn second() -> Enum {
    Enum::Second
}

#[kani::proof_for_contract(first)]
pub fn check_first() {
    let _ = first();
}

#[kani::proof_for_contract(second)]
pub fn check_second() {
    let _ = second();
}
