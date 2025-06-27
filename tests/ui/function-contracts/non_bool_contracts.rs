// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// This tests that Kani reports the "ideal" error message when contracts are non-boolean expressions
// By "ideal," we mean that the error spans are as narrow as possible
// (c.f. https://github.com/model-checking/kani/issues/3009)

#[kani::requires(a + b)]
#[kani::ensures(|result| a % *result && b % *result == 0 && *result != 0)]
fn gcd(a: u64, b: u64) -> u64 {
    0
}
