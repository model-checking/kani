// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

fn main() {
    let address = 0x01234usize;
    let ptr = address as *mut i32;
    // pointers can only be dereferenced inside unsafe blocks
    unsafe {
        // dereferencing a random address in memory will probably crash the program
        *ptr = 1; // rmc verification succeeds without generating any assertions
    };
}
