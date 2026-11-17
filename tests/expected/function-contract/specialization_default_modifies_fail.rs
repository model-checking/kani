// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: --edition 2021
// kani-flags: -Zfunction-contracts

#![feature(min_specialization)]

trait Probe {
    fn probe(allowed: &mut u8, forbidden: &mut u8);
}

impl<T> Probe for T {
    #[kani::modifies(allowed)]
    default fn probe(allowed: &mut u8, forbidden: &mut u8) {
        *allowed = 1;
        *forbidden = 2;
    }
}

impl Probe for u8 {
    fn probe(allowed: &mut u8, _forbidden: &mut u8) {
        *allowed = 3;
    }
}

#[kani::proof_for_contract(<u16 as Probe>::probe)]
fn check_frame_violation() {
    let mut allowed = 0;
    let mut forbidden = 0;
    <u16 as Probe>::probe(&mut allowed, &mut forbidden);
}
