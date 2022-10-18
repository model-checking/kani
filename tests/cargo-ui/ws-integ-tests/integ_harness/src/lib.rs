// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Dummy harness and library that check left rotation.

/// Return the right rotation of the give number and whether the result has wrapped bits.
pub fn rotate_right(num: u8, rhs: u32) -> (u8, bool) {
    let result = num.rotate_right(rhs);
    let wrapped = if rhs >= 8 { num != 0 } else { result < (num >> rhs) };
    (result, wrapped)
}
