// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

/// Checks that Kani catches an attempt to read uninitialized memory from a semi-initialized vector.
#[kani::proof]
fn check_vec_read_semi_init() {
    let mut v: Vec<u8> = Vec::with_capacity(10);
    unsafe { *v.as_mut_ptr().add(4) = 0x42 };
    let uninit = unsafe { *v.as_ptr().add(5) }; // ~ERROR: reading from unitialized memory is UB.
}
