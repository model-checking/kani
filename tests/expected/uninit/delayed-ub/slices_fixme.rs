// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

//! Checks that Kani catches instances of delayed UB for slices.
//! This test used to live inside delayed-ub, but the 2/5/2025 toolchain upgrade introduced a regression for this test.
//! Once this test is fixed, move it back into delayed-ub.rs
//! See https://github.com/model-checking/kani/issues/3881 for details.

/// Delayed UB via mutable pointer write into a slice element.
#[kani::proof]
fn delayed_ub_slices() {
    unsafe {
        // Create an array.
        let mut arr = [0u128; 4];
        // Get a pointer to a part of the array.
        let ptr = &mut arr[0..2][0..1][0] as *mut _ as *mut (u8, u32);
        *ptr = (4, 4);
        let arr_copy = arr; // UB: This reads a padding value inside the array!
    }
}
