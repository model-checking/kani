// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zghost-state -Zaliasing

// In the following code,
// ref_from_raw_1 and ref_from_raw_2
// borrow from the memory location of local.
// In the second iteration of the loop,
// the write to ref_from_raw_2 should end the scope
// of ref_from_raw_1's borrow.
// When ref_from_raw_1 then is written, an aliasing
// error should be thrown.

// This example is used as a litmus test to check
// that multiple basic blocks with non-linear
// control flow are instrumented properly

#[allow(unused)]
#[kani::proof]
fn violation_within_control_flow() {
    let mut local: i32 = 10;
    let mut referent_1: i32 = 0;
    let mut referent_2: i32 = 0;
    let mut ref_from_raw_1: &mut i32 = &mut referent_1;
    let mut ref_from_raw_2: &mut i32 = &mut referent_2;
    let raw_pointer: *mut i32 = &mut local as *mut i32;
    let mut state = false;
    let mut iters = 0;
    unsafe {
        while iters < 2 {
            if state {
                ref_from_raw_1 = &mut *raw_pointer;
                *ref_from_raw_1 = 0;
            } else {
                ref_from_raw_2 = &mut *raw_pointer;
                *ref_from_raw_2 = 1;
                *ref_from_raw_1 = 2;
            }
            state = true;
            iters += 1;
        }
    }
}
