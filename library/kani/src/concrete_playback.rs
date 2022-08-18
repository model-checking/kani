// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Helper code for concrete playback.

use std::cell::RefCell;

thread_local! {
    /// thread_local! gives us a separate CONCRETE_VALS instance for each thread.
    /// This allows us to run concrete playback unit tests in parallel.
    /// RefCell is necessary for mut statics.
    static CONCRETE_VALS: RefCell<Vec<Vec<u8>>> = RefCell::new(Vec::new());
}

/// This function sets concrete values and plays back the user's proof harness.
pub fn concrete_playback_run<F: Fn()>(mut local_concrete_vals: Vec<Vec<u8>>, proof_harness: F) {
    // Det vals in the user test case should be in the same order as the order of kani::any() calls.
    // Here, we need to reverse this order because det vals are popped off of the outer Vec,
    // so the chronological first det val should come last.
    local_concrete_vals.reverse();
    CONCRETE_VALS.with(|glob_concrete_vals| {
        let mut_ref_glob_concrete_vals = &mut *glob_concrete_vals.borrow_mut();
        *mut_ref_glob_concrete_vals = local_concrete_vals;
    });
    // Since F is a type argument, there should be a direct, static call to proof_harness().
    proof_harness();
    // This code will not run if a user assertion fails on concrete playback.
    // But if you comment out the failing assertion during playback,
    // this can be used to know if too many concrete values were loaded into the concrete playback test case.
    CONCRETE_VALS.with(|glob_concrete_vals| {
        let ref_glob_concrete_vals = &*glob_concrete_vals.borrow();
        assert!(
            ref_glob_concrete_vals.is_empty(),
            "At the end of concrete playback, there were still these concrete values left over `{:?}`. \
            This either happened because: \
            1) Your code/harness changed after you generated this concrete playback unit test. \
            2) There's a bug in Kani. Please report the issue here: <https://github.com/model-checking/kani/issues/new?assignees=&labels=bug&template=bug_report.md>",
            ref_glob_concrete_vals
        );
    });
}

/// Concrete playback implementation of kani::any_raw_internal.
///
/// # Safety
///
/// The semantics of this function require that SIZE_T equals the size of type T.
pub(crate) unsafe fn any_raw_internal<T, const SIZE_T: usize>() -> T {
    let mut next_concrete_val: Vec<u8> = Vec::new();
    CONCRETE_VALS.with(|glob_concrete_vals| {
        let mut_ref_glob_concrete_vals = &mut *glob_concrete_vals.borrow_mut();
        next_concrete_val = mut_ref_glob_concrete_vals.pop().expect("Not enough det vals found");
    });
    let next_concrete_val_len = next_concrete_val.len();
    let bytes_t: [u8; SIZE_T] = next_concrete_val.try_into().expect(&format!(
        "Expected {SIZE_T} bytes instead of {next_concrete_val_len} bytes in the following det vals vec"
    ));
    std::mem::transmute_copy::<[u8; SIZE_T], T>(&bytes_t)
}
