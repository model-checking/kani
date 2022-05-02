// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

#[kani::proof]
fn main() {
    let address = 0x01234usize;
    let ptr = address as *mut i32;
    // pointers can only be dereferenced inside unsafe blocks
    unsafe {
        // dereferencing a random address in memory will probably crash the program
        *ptr = 1; // kani verification succeeds without generating any assertions
    };
}
