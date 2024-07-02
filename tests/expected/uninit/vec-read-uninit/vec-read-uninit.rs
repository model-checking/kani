// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

/// Checks that Kani catches an attempt to read uninitialized memory from an uninitialized vector.
#[kani::proof]
fn check_vec_read_uninit() {
    let v: Vec<u8> = Vec::with_capacity(10);
    let uninit = unsafe { *v.as_ptr().add(5) }; // ~ERROR: reading from unitialized memory is UB.
}
