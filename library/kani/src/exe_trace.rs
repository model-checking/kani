// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Helper code for running executable traces.

use std::cell::RefCell;

thread_local! {
    /// thread_local! gives us a separate DET_VALS instance for each thread.
    /// This allows us to run deterministic unit tests in parallel.
    /// RefCell is necessary for mut statics.
    static DET_VALS: RefCell<Vec<Vec<u8>>> = RefCell::new(Vec::new());
}

/// This function sets deterministic values and plays back the user's proof harness.
pub fn exe_trace_run<F: Fn()>(mut local_det_vals: Vec<Vec<u8>>, proof_harness: F) {
    // Det vals in the user test case should be in the same order as the order of kani::any() calls.
    // Here, we need to reverse this order because det vals are popped off of the outer Vec,
    // so the chronological first det val should come last.
    local_det_vals.reverse();
    DET_VALS.with(|glob_det_vals| {
        let mut_ref_glob_det_vals = &mut *glob_det_vals.borrow_mut();
        *mut_ref_glob_det_vals = local_det_vals;
    });
    // Since F is a type argument, there should be a direct, static call to proof_harness().
    proof_harness();
    // This code will not run if a user assertion fails on deterministic playback.
    // But if you comment out the failing assertion during playback,
    // this can be used to know if too many det vals were loaded into the deterministic test case.
    DET_VALS.with(|glob_det_vals| {
        let ref_glob_det_vals = &*glob_det_vals.borrow();
        assert!(
            ref_glob_det_vals.is_empty(),
            "At the end of deterministic playback, there were still these deterministic values left over `{:?}`. \
            This either happened because: \
            1) Your code/harness changed after you generated this deterministic test. \
            2) There's a bug in Kani. Please report the issue here: <https://github.com/model-checking/kani/issues/new?assignees=&labels=bug&template=bug_report.md>",
            ref_glob_det_vals
        );
    });
}

/// Executable trace implementation of kani::any_raw_internal.
///
/// # Safety
///
/// The semantics of this function require that SIZE_T equals the size of type T.
pub(crate) unsafe fn any_raw_internal<T, const SIZE_T: usize>() -> T {
    let mut next_det_val: Vec<u8> = Vec::new();
    DET_VALS.with(|glob_det_vals| {
        let mut_ref_glob_det_vals = &mut *glob_det_vals.borrow_mut();
        next_det_val = mut_ref_glob_det_vals.pop().expect("Not enough det vals found");
    });
    let next_det_val_len = next_det_val.len();
    let bytes_t: [u8; SIZE_T] = next_det_val.try_into().expect(&format!(
        "Expected {SIZE_T} bytes instead of {next_det_val_len} bytes in the following det vals vec"
    ));
    std::mem::transmute_copy::<[u8; SIZE_T], T>(&bytes_t)
}
