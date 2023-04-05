// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks for error message with an --object-bits value that is too small

// kani-flags: --default-unwind 30 --enable-unstable --cbmc-args --object-bits 5

#[kani::proof]
fn main() {
    let mut arr: [i32; 100] = kani::Arbitrary::any_array();
    for i in 0..30 {
        arr[i] = kani::any();
    }
    assert!(arr[0] > arr[0] - arr[99]);
}
