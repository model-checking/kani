// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

//! This tests whether we properly replace missing functions with `assert(false)`

// https://doc.rust-lang.org/reference/items/external-blocks.html
// https://doc.rust-lang.org/nomicon/ffi.html
extern "C" {
    fn missing_int_converter(i: u32) -> u32;
}

#[kani::proof]
fn main() {
    unsafe {
        let x = missing_int_converter(3);
        assert!(x < 2 || x > 1);
    }
}
