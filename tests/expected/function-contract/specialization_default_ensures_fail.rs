// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: --edition 2021
// kani-flags: -Zfunction-contracts

#![feature(min_specialization)]

trait Probe {
    fn probe() -> u8;
}

impl<T> Probe for T {
    #[kani::ensures(|result| *result == 0)]
    default fn probe() -> u8 {
        1
    }
}

impl Probe for u8 {
    fn probe() -> u8 {
        0
    }
}

#[kani::proof_for_contract(<u16 as Probe>::probe)]
fn check_false_postcondition() {
    <u16 as Probe>::probe();
}
