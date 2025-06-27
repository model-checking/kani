// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Demonstrate that when the requires contract is before the ensures contract, the history expression respects the upper bound on x, so x + 1 doesn't overflow
// This example is taken from https://github.com/model-checking/kani/issues/3359

#[kani::requires(val < i32::MAX)]
#[kani::ensures(|result| *result == old(val + 1))]
pub fn next(val: i32) -> i32 {
    val + 1
}

#[kani::proof_for_contract(next)]
pub fn check_next() {
    let _ = next(kani::any());
}
