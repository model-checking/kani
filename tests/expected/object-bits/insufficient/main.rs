// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks for error message with an --object-bits value that is too small

// kani-flags: --default-unwind 30 --enable-unstable --cbmc-args --object-bits 5

#[kani::proof]
fn main() {
    let arr: [i32; 100] = kani::any();
    assert_eq!(arr[0], arr[99]);
}
