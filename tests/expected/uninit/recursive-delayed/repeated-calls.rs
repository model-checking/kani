// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// edition: 2021
// kani-flags: -Z ghost-state -Z uninit-checks

//! Checks memory initialization after repeated calls to a helper.

#[inline(never)]
fn write_tuple(value: *mut (u8, u32, u64)) {
    unsafe {
        *value = (1, 2, 3);
    }
}

#[kani::proof]
fn repeated_calls_uninitialized() {
    let mut first: u128 = 0;
    let mut second: u128 = 0;
    write_tuple((&mut first as *mut u128).cast());
    write_tuple((&mut second as *mut u128).cast());
    let _read = second;
}

#[kani::proof]
fn repeated_calls_initialized() {
    let mut value: u128 = 0;
    write_tuple((&mut value as *mut u128).cast());
    write_tuple((&mut value as *mut u128).cast());
    value = 42;
    assert_eq!(value, 42);
}
