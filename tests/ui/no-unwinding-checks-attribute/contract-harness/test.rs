// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Checks that `#[kani::no_unwinding_checks]` is rejected on a `proof_for_contract` harness.
//! The contract of `count_to` is false for `n >= 3`, which a proof that stops at the unwinding
//! bound would not see, and `stub_verified` would then trust the contract beyond the bound.

#[kani::requires(n < 10)]
#[kani::ensures(|result: &u8| *result < 3)]
fn count_to(n: u8) -> u8 {
    let mut count = 0;
    while count < n {
        count += 1;
    }
    count
}

#[kani::proof_for_contract(count_to)]
#[kani::unwind(3)]
#[kani::no_unwinding_checks]
fn check_count_to() {
    count_to(kani::any());
}
