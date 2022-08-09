// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Helper code for running executable traces.

use std::cell::RefCell;

/// thread_local! gives us a separate DET_VALS instance for each thread.
/// This allows us to run deterministic unit tests in parallel.
/// RefCell is necessary for mut statics.
thread_local! {
    static DET_VALS: RefCell<Vec<Vec<u8>>> = RefCell::new(Vec::new());
}

/// This function sets deterministic values and plays back the user's proof harness.
pub fn exe_trace_run<F: Fn()>(mut det_vals: Vec<Vec<u8>>, proof_harness: F) {
    // Det vals in the user test case should be in the same order as the order of kani::any() calls.
    // Here, we need to reverse this order because det vals are popped off of the outer Vec,
    // so the chronological first det val should come last.
    det_vals.reverse();
    DET_VALS.with(|det_vals| {
        let mut_ref_det_vals = &mut *det_vals.borrow_mut();
        *mut_ref_det_vals = det_vals;
    });
    // Since F is a type argument, there should be a direct, static call to proof_harness().
    proof_harness();
}

/// Executable trace implementation of kani::any_raw_internal.
pub unsafe fn any_raw_internal<T, const SIZE_T: usize>() -> T {
    let mut next_det_val: Vec<Vec<u8>> = Vec::new();
    DET_VALS.with(|det_vals| {
        let mut_ref_det_vals = &mut &det_vals.borrow_mut();
        next_det_val = mut_ref_det_vals.pop().expect("Not enough det vals found");
    });
    let next_det_val_len = next_det_val.len();
    let bytes_t: [u8; SIZE_T] = next_det_val.try_into().expect(&format!(
        "Expected {SIZE_T} bytes instead of {next_det_val_len} bytes in the following det vals vec"
    ));
    return std::mem::transmute_copy::<[u8; SIZE_T], T>(&bytes_t);
}
