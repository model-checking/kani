// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that `old()` is executed after the pre-conditions, otherwise it can fail.
//!
//! Issue: <https://github.com/model-checking/kani/issues/3370>
// kani-flags: -Zfunction-contracts

#[kani::requires(val < i32::MAX)]
#[kani::ensures(|result| *result == old(val + 1))]
pub fn next(mut val: i32) -> i32 {
    val + 1
}

#[kani::proof_for_contract(next)]
pub fn check_next() {
    // let _ = next(kani::any_where(|val: &i32| *val < i32::MAX));
    let _ = next(kani::any());
}
