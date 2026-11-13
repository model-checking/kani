// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: --edition 2021
// kani-flags: -Zfunction-contracts -Zstubbing

#![feature(min_specialization)]

trait Probe {
    fn requires_only(value: u8) -> u8;
    fn ensures_only() -> u8;
    fn modifies_only(value: &mut u8);
    fn requires_first(value: &mut u8);
    fn modifies_first(value: &mut u8);
}

impl<T> Probe for T {
    #[kani::requires(value < 10)]
    default fn requires_only(value: u8) -> u8 {
        value + 1
    }

    #[kani::ensures(|result| *result == 7)]
    default fn ensures_only() -> u8 {
        7
    }

    #[kani::modifies(value)]
    default fn modifies_only(value: &mut u8) {
        *value = 7;
    }

    #[kani::requires(*value < 10)]
    #[kani::ensures(|_| *value == 7)]
    #[kani::modifies(value)]
    default fn requires_first(value: &mut u8) {
        *value = 7;
    }

    #[kani::modifies(value)]
    #[kani::ensures(|_| *value == 7)]
    #[kani::requires(*value < 10)]
    default fn modifies_first(value: &mut u8) {
        *value = 7;
    }
}

impl Probe for u8 {
    fn requires_only(value: u8) -> u8 {
        value
    }
    fn ensures_only() -> u8 {
        9
    }
    fn modifies_only(value: &mut u8) {
        *value = 9;
    }
    fn requires_first(value: &mut u8) {
        *value = 9;
    }
    fn modifies_first(value: &mut u8) {
        *value = 9;
    }
}

#[kani::proof_for_contract(<u16 as Probe>::requires_only)]
fn check_requires() {
    <u16 as Probe>::requires_only(kani::any());
}

#[kani::proof_for_contract(<u16 as Probe>::ensures_only)]
fn check_ensures() {
    <u16 as Probe>::ensures_only();
}

#[kani::proof_for_contract(<u16 as Probe>::modifies_only)]
fn check_modifies() {
    let mut value = kani::any();
    <u16 as Probe>::modifies_only(&mut value);
}

#[kani::proof_for_contract(<u16 as Probe>::requires_first)]
fn check_requires_first() {
    let mut value = kani::any();
    <u16 as Probe>::requires_first(&mut value);
}

#[kani::proof_for_contract(<u16 as Probe>::modifies_first)]
fn check_modifies_first() {
    let mut value = kani::any();
    <u16 as Probe>::modifies_first(&mut value);
}

#[kani::proof]
#[kani::stub_verified(<u16 as Probe>::requires_first)]
fn stub_requires_first() {
    let mut value = 0;
    <u16 as Probe>::requires_first(&mut value);
    assert_eq!(value, 7);
}

#[kani::proof]
#[kani::stub_verified(<u16 as Probe>::modifies_first)]
fn stub_modifies_first() {
    let mut value = 0;
    <u16 as Probe>::modifies_first(&mut value);
    assert_eq!(value, 7);
}
