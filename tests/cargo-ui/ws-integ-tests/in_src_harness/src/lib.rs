// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Dummy harness and library that check left rotation.

/// Return the left rotation of the give number and whether the result had wrapped bits.
pub fn rotate_left(num: u8, rhs: u32) -> (u8, bool) {
    let result = num.rotate_left(rhs);
    let wrapped = if rhs >= 8 { num != 0 } else { result > (num << rhs) };
    (result, wrapped)
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn will_not_panic() {
        rotate_left(kani::any(), kani::any());
    }
}
