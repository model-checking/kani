// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Helper code for running executable traces.

use std::sync::Mutex;
/// DET_VALS_LOCK is used by each playback test case to ensure that only a single thread is modifying DET_VALS at once.
/// We need to separate the lock from the data because there's no other way to pass the data from
/// kani::exe_trace_run() to kani::any_raw_internal() while still holding the lock.
static DET_VALS_LOCK: Mutex<()> = Mutex::new(());
pub static mut DET_VALS: Vec<Vec<u8>> = Vec::new();

/// This function sets deterministic values and plays back the user's proof harness.
pub fn exe_trace_run<F: Fn()>(mut det_vals: Vec<Vec<u8>>, proof_harness: F) {
    // Det vals in the user test case should be in the same order as the order of kani::any() calls.
    // Here, we need to reverse this order because det vals are popped off of the outer Vec,
    // so the chronological first det val should come last.
    det_vals.reverse();
    // If another thread panicked while holding the lock (e.g., because they hit an expected assertion failure), we still want to continue.
    let _guard = match DET_VALS_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    unsafe {
        DET_VALS = det_vals;
    }
    // Since F is a type argument, there should be a direct, static call to proof_harness().
    proof_harness();
}

/// Executable trace implementation of kani::any_raw_internal.
pub unsafe fn any_raw_internal<T, const SIZE_T: usize>() -> T {
    // This code will only run when our thread's exe_trace_run() fn holds the lock.
    let next_det_val = DET_VALS.pop().expect("Not enough det vals found");
    let next_det_val_len = next_det_val.len();
    let bytes_t: [u8; SIZE_T] = next_det_val.try_into().expect(&format!(
        "Expected {SIZE_T} bytes instead of {next_det_val_len} bytes in the following det vals vec"
    ));
    return std::mem::transmute_copy::<[u8; SIZE_T], T>(&bytes_t);
}
