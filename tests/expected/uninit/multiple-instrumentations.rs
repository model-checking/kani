// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

/// Ensure instrumentation works correctly when a single instruction gets multiple instrumentations.
#[kani::proof]
fn multiple_instrumentations() {
    unsafe {
        let mut value: u128 = 0;
        // Cast between two pointers of different padding.
        let ptr = &mut value as *mut _ as *mut (u8, u32, u64);
        *ptr = (4, 4, 4);
        // Here, instrumentation is added 2 times before the function call and 1 time after.
        value = helper_1(value, value);
    }
}

fn helper_1(a: u128, b: u128) -> u128 {
    a + b
}

/// Ensure instrumentation works correctly when a single instruction gets multiple instrumentations
/// (and variables are different).
#[kani::proof]
fn multiple_instrumentations_different_vars() {
    unsafe {
        let mut a: u128 = 0;
        let mut b: u64 = 0;
        // Cast between two pointers of different padding.
        let ptr_a = &mut a as *mut _ as *mut (u8, u32, u64);
        *ptr_a = (4, 4, 4);
        // Cast between two pointers of different padding.
        let ptr_b = &mut b as *mut _ as *mut (u8, u32);
        *ptr_b = (4, 4);
        // Here, instrumentation is added 2 times before the function call and 1 time after.
        a = helper_2(a, b);
    }
}

fn helper_2(a: u128, b: u64) -> u128 {
    a + (b as u128)
}
