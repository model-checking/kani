// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: --edition 2021
// kani-flags: -Zfunction-contracts -Zstubbing --no-assert-contracts

#![feature(min_specialization)]

trait Probe {
    fn probe(value: u8) -> u8;
}

impl<T> Probe for T {
    #[kani::requires(value != 0)]
    default fn probe(value: u8) -> u8 {
        value
    }
}

impl Probe for u8 {
    fn probe(value: u8) -> u8 {
        value
    }
}

// Verified stubs require a proof harness for the contract they use.
#[kani::proof_for_contract(<u16 as Probe>::probe)]
fn check_contract() {
    <u16 as Probe>::probe(kani::any());
}

#[kani::proof]
#[kani::stub_verified(<u16 as Probe>::probe)]
fn check_invalid_caller() {
    <u16 as Probe>::probe(0);
}
