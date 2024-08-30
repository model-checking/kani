// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zghost-state -Zaliasing

// In the following code,
// ref_from_raw_1 and ref_from_raw_2 both
// borrow the memory location of local.
// After ref_from_raw_2 is written, ref_from_raw_1's
// borrow ends.
// The subsequent write to ref_from_raw_1 will cause an aliasing
// error.

#[kani::proof]
fn use_after_borrow_ends() {
    let mut local: i32;
    let temp_ref: &mut i32;
    let raw_pointer: *mut i32;
    let ref_from_raw_1: &mut i32;
    let ref_from_raw_2: &mut i32;

    local = 0;
    temp_ref = &mut local;
    raw_pointer = temp_ref as *mut i32;
    unsafe {
        ref_from_raw_1 = &mut *raw_pointer;
        *ref_from_raw_1 = 0;
        ref_from_raw_2 = &mut *raw_pointer;
        *ref_from_raw_2 = 1;
        *ref_from_raw_1 = 2;
    }
}
