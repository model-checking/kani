// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Edited version for the proof diff test in `test_scripts.py`. Not compiled.

#[kani::requires(d != 0 && n > 3)]
pub fn div(n: u32, d: u32) -> u32 {
    // A comment-only change is not a code change.
    n / d
}

pub fn helper(x: u32) -> u32 {
    x.saturating_add(1)
}

pub fn added() {}

impl Thing {
    fn method(&self) -> u8 {
        1
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    fn kani_only_helper() -> u8 {
        2
    }

    #[kani::proof]
    #[kani::unwind(2)]
    #[kani::stub(helper, fake_helper)]
    #[kani::should_panic]
    fn check_helper() {
        let x: u32 = kani::any();
        kani::assume(x < 10);
        kani::assume(x != 3);
    }

    #[kani::proof]
    fn check_new() {}
}
