// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test case checks the behavior of `size_of_val_raw` for dynamically sized types.

#![feature(layout_for_ptr)]
#![feature(ptr_metadata)]

use std::mem::size_of_val_raw;
use std::ptr;

#[derive(Clone, Copy, kani::Arbitrary)]
struct Wrapper<T: ?Sized> {
    _size: usize,
    _value: T,
}

#[kani::proof]
pub fn check_size_of_adt_overflows() {
    let var: Wrapper<[u64; 4]> = kani::any();
    let fat_ptr: *const Wrapper<[u64]> = &var as *const _;
    let (thin_ptr, size) = fat_ptr.to_raw_parts();
    let new_size: usize = kani::any();
    let new_ptr: *const Wrapper<[u64]> = ptr::from_raw_parts(thin_ptr, new_size);
    if let Some(slice_size) = new_size.checked_mul(size_of::<u64>()) {
        if let Some(expected_size) = slice_size.checked_add(size_of::<usize>()) {
            assert_eq!(unsafe { size_of_val_raw(new_ptr) }, expected_size);
        } else {
            // Expect UB detection
            let _should_ub = unsafe { size_of_val_raw(new_ptr) };
            kani::cover!(true, "Expected unreachable");
        }
    } else {
        // Expect UB detection
        let _should_ub = unsafe { size_of_val_raw(new_ptr) };
        kani::cover!(true, "Expected unreachable");
    }
}

#[kani::proof]
pub fn check_size_of_overflows() {
    let var: [u64; 4] = kani::any();
    let fat_ptr: *const [u64] = &var as *const _;
    let (thin_ptr, size) = fat_ptr.to_raw_parts();
    let new_size: usize = kani::any();
    let new_ptr: *const [u64] = ptr::from_raw_parts(thin_ptr, new_size);
    if let Some(expected_size) = new_size.checked_mul(size_of::<u64>()) {
        assert_eq!(unsafe { size_of_val_raw(new_ptr) }, expected_size);
    } else {
        // Expect UB detection
        let _should_ub = unsafe { size_of_val_raw(new_ptr) };
    }
}
