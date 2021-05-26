// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
    let address = 0x01234usize;
    let ptr = address as *mut i32;
    unsafe {
        // dereferencing a random address in memory will probably crash the program
        *ptr = 1;
    };
}
