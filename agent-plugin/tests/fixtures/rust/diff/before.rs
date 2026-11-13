// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Baseline for the proof diff test in `test_scripts.py`. Not compiled.

#[kani::requires(d != 0)]
#[kani::ensures(|r: &u32| *r <= n)]
pub fn div(n: u32, d: u32) -> u32 {
    n / d
}

pub fn helper(x: u32) -> u32 {
    x + 1
}

pub fn removed_later() {}

impl Thing {
    fn method(&self) -> u8 {
        1
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    fn kani_only_helper() -> u8 {
        1
    }

    #[kani::proof]
    #[kani::unwind(8)]
    fn check_helper() {
        let x: u32 = kani::any();
        kani::assume(x < 100);
        assert!(helper(x) > x);
        kani::cover!(true, "end");
    }

    #[kani::proof]
    fn check_gone() {}
}
