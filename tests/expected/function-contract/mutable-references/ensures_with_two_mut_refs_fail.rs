// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

//! When a contract returns a mutable reference to one of its arguments, we cannot
//! use the `ensures` parameter alongside the function's argument, as they are two
//! mutable references to the same data.

#[kani::ensures(|result| **result == 42 && **result == *n)]
#[kani::modifies(n)]
fn forty_two(n: &mut u8) -> &mut u8 {
    *n = 42;
    n
}

#[kani::proof_for_contract(forty_two)]
fn forty_two_harness() {
    forty_two(&mut kani::any());
}
