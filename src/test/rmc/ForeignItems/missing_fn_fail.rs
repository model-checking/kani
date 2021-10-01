// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This tests whether we properly replace missing functions with `assert(false)`
//! To run this test, do
//! rmc missing_fn_fail.rs -- lib.c

// rmc-flags: --c-lib src/test/rmc/ForeignItems/lib.c

// https://doc.rust-lang.org/reference/items/external-blocks.html
// https://doc.rust-lang.org/nomicon/ffi.html
extern "C" {
    fn missing_int_converter(i: u32) -> u32;
}

pub fn main() {
    unsafe {
        let x = missing_int_converter(3);
        assert!(x < 2 || x > 1);
    }
}
