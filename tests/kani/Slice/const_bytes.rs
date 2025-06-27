// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that byte slices are codegen correctly. This used to fail
//! in the past (see https://github.com/model-checking/kani/issues/2656).

#[kani::proof]
fn main() {
    const MY_CONSTANT: &[u8] = &[147, 211];
    let x: u8 = MY_CONSTANT[0];
    let y: u8 = MY_CONSTANT[1];
    assert_eq!(x, 147);
    assert_eq!(y, 211);
}
