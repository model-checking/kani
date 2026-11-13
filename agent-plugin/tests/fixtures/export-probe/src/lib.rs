// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A crate for checking that the plugin reads Kani's `--export-json` output the same way on
//! every Kani build: one harness with an unreachable assertion and an unsatisfiable cover, one
//! that fails, and one with a stub and a verified stub.

pub fn double(x: u8) -> u8 {
    x.wrapping_mul(2)
}

pub fn expensive(x: u8) -> u8 {
    x / 2
}

#[cfg(kani)]
mod proofs {
    use super::*;

    fn stub_expensive(x: u8) -> u8 {
        x / 2
    }

    #[kani::requires(x < 100)]
    #[kani::ensures(|r| *r == x * 2)]
    fn checked_double(x: u8) -> u8 {
        x * 2
    }

    #[kani::proof]
    fn passes() {
        let x: u8 = kani::any();
        kani::assume(x < 10);
        if x >= 10 {
            assert!(false, "cannot happen");
        }
        kani::cover!(x > 20, "impossible");
        assert!(double(x) < 20);
    }

    #[kani::proof]
    fn fails() {
        let x: u8 = kani::any();
        assert!(x + 1 > x, "overflow when x is u8::MAX");
    }

    #[kani::proof_for_contract(checked_double)]
    fn check_checked_double() {
        let x: u8 = kani::any();
        checked_double(x);
    }

    #[kani::proof]
    #[kani::stub(expensive, stub_expensive)]
    #[kani::stub_verified(checked_double)]
    fn stubbed() {
        let x: u8 = kani::any();
        kani::assume(x < 50);
        let _ = expensive(x);
        let _ = checked_double(x);
    }
}
