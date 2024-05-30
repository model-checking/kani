// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#[kani::ensures(|result| (result == x) | (result == y))]
fn max(x: u32, y: u32) -> u32 {
    if x > y { x } else { y }
}

#[kani::proof_for_contract(max)]
fn max_harness() {
    let _ = Box::new(9_usize);
    max(7, 6);
}
