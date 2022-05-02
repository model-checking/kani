// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `transmute` works as expected when turning a `str` into
// `&[u8]`.

// This test is a modified version of the example found in
// https://doc.rust-lang.org/std/intrinsics/fn.transmute.html

#[kani::proof]
fn main() {
    let slice = unsafe { std::mem::transmute::<&str, &[u8]>("Rust") };
    assert_eq!(slice, &[82, 117, 115, 116]);
}
