// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zghost-state -Zaliasing

// The following code is equivalent to duplicate_write,
// the only difference being that operations may be chained
// in one line.

#[kani::proof]
fn main() {
    let mut local: i32 = 0;
    let raw_pointer = &mut local as *mut i32;
    unsafe {
        let ref_from_raw_1 = &mut *raw_pointer;
        *ref_from_raw_1 = 0;
        let ref_from_raw_2 = &mut *raw_pointer;
        *ref_from_raw_2 = 1;
        *ref_from_raw_1 = 2;
    }
}
