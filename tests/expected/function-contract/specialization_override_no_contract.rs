// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: --edition 2021
// kani-flags: -Zfunction-contracts

#![feature(min_specialization)]

trait Probe {
    fn probe() -> u8;
}

impl<T> Probe for T {
    #[kani::ensures(|result| *result == 7)]
    default fn probe() -> u8 {
        7
    }
}

impl Probe for u8 {
    fn probe() -> u8 {
        9
    }
}

#[kani::proof_for_contract(<u8 as Probe>::probe)]
fn check_override_without_contract() {
    assert_eq!(<u8 as Probe>::probe(), 9);
}
