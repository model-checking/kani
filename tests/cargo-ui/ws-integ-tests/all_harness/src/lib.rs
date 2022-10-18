// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Dummy harness and library that check left rotation.
use in_src_harness::rotate_left;
use integ_harness::rotate_right;

/// Returns if any rotation would result in wrapping bits.
pub fn will_rotate_wrap(num: u8, rhs: u32) -> bool {
    rotate_left(num, rhs).1 || rotate_right(num, rhs).1
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn check_propagate() {
        let num = kani::any();
        let shift = kani::any();
        kani::assume(rotate_right(num, shift).1);
        assert!(will_rotate_wrap(num, shift));
    }
}
