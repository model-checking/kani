// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: --edition 2021
// kani-flags: -Zfunction-contracts -Zstubbing

#![feature(min_specialization)]

trait Probe {
    fn increment(value: &mut u8);
}

impl<T> Probe for T {
    #[kani::ensures(|_| *value == old(*value) + 1)]
    #[kani::requires(*value < 10)]
    #[kani::modifies(value)]
    default fn increment(value: &mut u8) {
        *value += 1;
    }
}

impl Probe for u8 {
    fn increment(value: &mut u8) {
        *value = 9;
    }
}

#[kani::proof_for_contract(<u16 as Probe>::increment)]
fn check_eager_old() {
    let mut value = kani::any();
    <u16 as Probe>::increment(&mut value);
}

#[kani::proof]
#[kani::stub_verified(<u16 as Probe>::increment)]
fn stub_eager_old() {
    let mut value = 3;
    <u16 as Probe>::increment(&mut value);
    assert_eq!(value, 4);
}
