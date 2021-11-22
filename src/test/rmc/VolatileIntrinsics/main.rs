// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]

use std::mem;
use std::ptr;
// https://doc.rust-lang.org/std/ptr/fn.write_volatile.html
fn test_volatile_store() {
    let mut x = 0;
    let y = &mut x as *mut i32;
    let z = 12;

    unsafe {
        std::ptr::write_volatile(y, z);
        assert!(std::ptr::read_volatile(y) == 12);
    }
}

// https://doc.redox-os.org/std/std/intrinsics/fn.volatile_copy_memory.html
fn test_copy_volatile() {
    // TODO: make an overlapping set of locations, and check that it does the right thing for the overlapping region too.
    // https://github.com/model-checking/rmc/issues/12
    let mut expected_val = 42;
    let src: *mut i32 = &mut expected_val as *mut i32;
    let mut old_val = 99;
    let dst: *mut i32 = &mut old_val;
    unsafe {
        std::intrinsics::volatile_copy_memory(src, dst, 1);
        assert!(*dst == expected_val);
    }
}

/// https://doc.rust-lang.org/std/ptr/fn.copy_nonoverlapping.html
/// Moves all the elements of `src` into `dst`, leaving `src` empty.
fn append<T>(dst: &mut Vec<T>, src: &mut Vec<T>) {
    let src_len = src.len();
    let dst_len = dst.len();

    // Ensure that `dst` has enough capacity to hold all of `src`.
    dst.reserve(src_len);

    unsafe {
        // The call to offset is always safe because `Vec` will never
        // allocate more than `isize::MAX` bytes.
        let dst_ptr = dst.as_mut_ptr().offset(dst_len as isize);
        let src_ptr = src.as_mut_ptr();

        // Truncate `src` without dropping its contents. We do this first,
        // to avoid problems in case something further down panics.
        src.set_len(0);

        // The two regions cannot overlap because mutable references do
        // not alias, and two different vectors cannot own the same
        // memory.
        std::intrinsics::volatile_copy_nonoverlapping_memory(src_ptr, dst_ptr, src_len);

        // Notify `dst` that it now holds the contents of `src`.
        dst.set_len(dst_len + src_len);
    }
}

fn test_append() {
    let mut a = vec!['r'];
    let mut b = vec!['u', 's', 't'];

    append(&mut a, &mut b);

    assert!(a == &['r', 'u', 's', 't']);
    assert!(b.is_empty());
}

/// Test the swap example from https://doc.redox-os.org/std/std/intrinsics/fn.copy_nonoverlapping.html
/// Using T as uninitialized as in the example gives other errors, which we will solve later.
/// For this test, passing in a default value that gets overridden is sufficient.
fn swap_with_default<T>(x: &mut T, y: &mut T, default: T) {
    unsafe {
        // Give ourselves some scratch space to work with
        //         let mut t: T = mem::uninitialized();
        let mut t: T = default;

        // Perform the swap, `&mut` pointers never alias
        ptr::copy_nonoverlapping(x, &mut t, 1);
        ptr::copy_nonoverlapping(y, x, 1);
        ptr::copy_nonoverlapping(&t, y, 1);

        // y and t now point to the same thing, but we need to completely forget `tmp`
        // because it's no longer relevant.
        mem::forget(t);
    }
}

/// another test for copy_nonoverlapping, from
/// https://doc.redox-os.org/std/std/intrinsics/fn.copy_nonoverlapping.html
fn test_swap() {
    let mut x = 12;
    let mut y = 13;
    swap_with_default(&mut x, &mut y, 3);
    assert!(x == 13);
    assert!(y == 12);
}

/// https://doc.redox-os.org/std/std/intrinsics/fn.copy_nonoverlapping.html
/// https://doc.redox-os.org/std/std/intrinsics/fn.volatile_copy_nonoverlapping_memory.html
fn test_copy_volatile_nonoverlapping() {
    // TODO: make an overlapping set of locations, and check that it does the right thing for the overlapping region too.
    // https://github.com/model-checking/rmc/issues/12
    let mut expected_val = 42;
    let src: *mut i32 = &mut expected_val as *mut i32;
    let mut old_val = 99;
    let dst: *mut i32 = &mut old_val;
    unsafe {
        std::intrinsics::volatile_copy_nonoverlapping_memory(src, dst, 1);
        assert!(*dst == expected_val);
    }
}

fn main() {
    test_volatile_store();
    test_copy_volatile();
    test_copy_volatile_nonoverlapping();
    test_swap();
    test_append();
}
