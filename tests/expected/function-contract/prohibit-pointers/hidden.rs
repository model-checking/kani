// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#![allow(unreachable_code, unused_variables)]

struct HidesAPointer(*mut u32);

#[kani::ensures(true)]
fn hidden_pointer(h: HidesAPointer) {}

#[kani::proof_for_contract(hidden_pointer)]
fn harness() {
    let mut a = 0;
    hidden_pointer(HidesAPointer(&mut a))
}
