// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the performance of calling format.
//! This tests capture the performance regression introduced by the toolchain upgrade #2551.
//! See https://github.com/model-checking/kani/issues/2576 for more details.

#[kani::proof]
fn fmt_i8() {
    let num: i8 = kani::any();
    let s = format!("{num}");
    assert!(s.len() <= 4);
}

#[kani::proof]
fn fmt_u8() {
    let num: u8 = kani::any();
    let s = format!("{num}");
    assert!(s.len() <= 3);
}
