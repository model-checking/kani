// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zautomatic-harnesses -Zfunction-contracts

// The automatic harnesses feature will only generate standard harnesses (#[kani::proof])
// for each function, even if it has a contract.
// That standard harness will ignore the function's contract,
// causing verification failure in this case (division by zero).
// Instead, we should detect the presence of contracts and generate a contract harness.

#[kani::requires(divisor != 0)]
fn div(dividend: u32, divisor: u32) -> u32 {
    dividend / divisor
}

// TODO add more tests, including test(s) for unsafe functions.
