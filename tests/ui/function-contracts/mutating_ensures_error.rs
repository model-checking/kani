// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#[kani::ensures(|_| {*_x += 1; true})]
fn unit(_x: &mut u32) {}

#[kani::proof_for_contract(id)]
fn harness() {
    let mut x = kani::any();
    unit(&mut x);
}
