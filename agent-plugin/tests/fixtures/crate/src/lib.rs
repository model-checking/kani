// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fixture crate for the agent plugin smoke tests (`tests/agent-plugin`). It has one harness
//! of each kind the skill must handle: a meaningful proof, a real bug, and a vacuous proof.

/// Midpoint of `a` and `b`, rounded down, without overflow.
pub fn midpoint(a: u32, b: u32) -> u32 {
    a / 2 + b / 2 + (a % 2 + b % 2) / 2
}

/// Midpoint of `a` and `b`. Overflows when `a + b > u32::MAX`.
pub fn naive_midpoint(a: u32, b: u32) -> u32 {
    (a + b) / 2
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn check_midpoint() {
        let a: u32 = kani::any();
        let b: u32 = kani::any();
        let m = midpoint(a, b);
        assert!(a.min(b) <= m && m <= a.max(b));
        kani::cover!(true, "harness end reached");
    }

    #[kani::proof]
    fn check_naive_midpoint() {
        let a: u32 = kani::any();
        let b: u32 = kani::any();
        let m = naive_midpoint(a, b);
        assert!(a.min(b) <= m && m <= a.max(b));
    }

    #[kani::proof]
    fn check_vacuous() {
        let a: u32 = kani::any();
        kani::assume(a > 10);
        kani::assume(a < 5);
        assert!(midpoint(a, a) == a);
    }
}

/// Integer division with a contract.
#[cfg_attr(kani, kani::requires(divisor != 0))]
#[cfg_attr(kani, kani::requires(dividend < 1000 || dividend >= 0))]
pub fn checked_div(dividend: u32, divisor: u32) -> u32 {
    dividend / divisor
}

/// A contract no input satisfies.
#[cfg_attr(kani, kani::requires(x > 10 && x < 5))]
pub fn impossible(x: u8) -> u8 {
    x
}

#[cfg(kani)]
mod contract_proofs {
    use super::*;

    #[kani::proof_for_contract(checked_div)]
    fn check_div_contract() {
        checked_div(kani::any(), kani::any());
    }
}
