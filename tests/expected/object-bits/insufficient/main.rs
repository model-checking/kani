// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --default-unwind 30 --enable-unstable --cbmc-args --object-bits 5
//! Checks for error message with an --object-bits value that is too small
//! Use linked list to ensure that each member represents a new object.

#[kani::proof]
fn main() {
    let arr: [i32; 18] = kani::Arbitrary::any_array();
    std::hint::black_box(std::collections::LinkedList::from(arr));
}
