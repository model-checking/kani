// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts
//! Test that contracts supports function args with pattern use and mut declaration.

#[kani::ensures(|result : &u32| *result <= dividend)]
fn div((mut dividend, divisor): (u32, u32)) -> u32 {
    dividend / divisor
}

#[kani::proof_for_contract(div)]
fn div_harness() {
    let _ = Box::new(());
    div((kani::any(), kani::any()));
}
