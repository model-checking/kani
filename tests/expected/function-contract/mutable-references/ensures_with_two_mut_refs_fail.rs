// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

#[kani::ensures(|result| **result == 42 && *result == n)]
#[kani::modifies(n)]
fn forty_two(n: &mut u8) -> &mut u8 {
    *n = 42;
    n
}

#[kani::proof_for_contract(forty_two)]
fn forty_two_harness() {
    forty_two(&mut kani::any());
}
