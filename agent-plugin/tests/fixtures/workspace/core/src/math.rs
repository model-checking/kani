// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Midpoint without overflow.
pub fn midpoint(a: u32, b: u32) -> u32 {
    a / 2 + b / 2 + (a % 2 + b % 2) / 2
}

/// A function with a contract.
#[cfg_attr(kani, kani::requires(d != 0))]
#[cfg_attr(kani, kani::ensures(|r: &u32| *r <= n))]
pub fn div(n: u32, d: u32) -> u32 {
    n / d
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
    }

    #[kani::proof_for_contract(div)]
    fn check_div_contract() {
        let _ = div(kani::any(), kani::any());
    }
}
