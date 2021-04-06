// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! To run this test, do
//! rmc fixme_varadic.rs -- lib.c

use std::os::raw::c_int;

// https://doc.rust-lang.org/reference/items/external-blocks.html
// https://doc.rust-lang.org/nomicon/ffi.html
extern "C" {
    fn my_add(num: usize, ...) -> usize;
    fn my_add2(num: usize, ...) -> c_int;

}

fn main() {
    unsafe {
        assert!(my_add(2 as usize, 3 as usize, 4 as usize) == 7); //works
        assert!(my_add(3, 3 as usize, 4 as usize, 5 as usize) == 12); //works
        assert!(my_add2(2, -1 as c_int, -3 as c_int) == -4); //works
    }
}
