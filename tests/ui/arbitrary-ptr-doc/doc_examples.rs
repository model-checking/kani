// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z mem-predicates

//! These are copies of the examples we added to our documentation.
//! We currently cannot run our examples using Kani.
//! We may be able to leverage `--runtool` from rustdoc once its stabilized. See
//! <https://github.com/rust-lang/rust/issues/64245>.
#![allow(unused)]

extern crate kani;

use kani::*;
#[kani::proof]
fn basic_inbounds() {
    let mut generator = PointerGenerator::<10>::new();
    let arbitrary = generator.any_alloc_status::<char>();
    kani::assume(arbitrary.status == AllocationStatus::InBounds);
    // Pointer may be unaligned, but it should be in-bounds, so it is safe to write to
    unsafe { arbitrary.ptr.write_unaligned(kani::any()) }
}

#[kani::proof]
fn same_capacity() {
    // These generators have the same capacity of 6 bytes.
    let generator1 = PointerGenerator::<6>::new();
    let generator2 = pointer_generator::<i16, 3>();
}

#[kani::proof]
fn generator_large_enough() {
    let mut generator = PointerGenerator::<6>::new();
    let ptr1: *mut u8 = generator.any_in_bounds().ptr;
    let ptr2: *mut u8 = generator.any_in_bounds().ptr;
    let ptr3: *mut u32 = generator.any_in_bounds().ptr;
    // This cover is satisfied.
    cover!(
        (ptr1 as usize) >= (ptr2 as usize) + size_of::<u8>()
            && (ptr2 as usize) >= (ptr3 as usize) + size_of::<u32>()
    );
    // As well as having overlapping pointers.
    cover!((ptr1 as usize) == (ptr3 as usize));
}

#[kani::proof]
fn pointer_may_be_same() {
    let mut generator = pointer_generator::<char, 5>();
    let ptr1 = generator.any_in_bounds::<char>().ptr;
    let ptr2 = generator.any_in_bounds::<char>().ptr;
    // This cover is satisfied.
    cover!(ptr1 == ptr2)
}
unsafe fn my_target<T>(_ptr1: *const T, _ptr2: *const T) {}

#[kani::proof]
fn usage_example() {
    let mut generator1 = pointer_generator::<char, 5>();
    let mut generator2 = pointer_generator::<char, 5>();
    let ptr1: *const char = generator1.any_in_bounds().ptr;
    let ptr2: *const char = if kani::any() {
        // Pointers will have same provenance and may overlap.
        generator1.any_in_bounds().ptr
    } else {
        // Pointers will have different provenance and will not overlap.
        generator2.any_in_bounds().ptr
    };
    // Invoke the function under verification
    unsafe { my_target(ptr1, ptr2) };
}
#[kani::proof]
fn diff_from_usize() {
    // This pointer represents any address, and it may point to anything in memory,
    // allocated or not.
    let ptr1 = kani::any::<usize>() as *const u8;

    // This pointer address will either point to unallocated memory, to a dead object
    // or to allocated memory within the generator address space.
    let mut generator = PointerGenerator::<5>::new();
    let ptr2: *const u8 = generator.any_alloc_status().ptr;
}
#[kani::proof]
fn check_distance() {
    let mut generator = PointerGenerator::<6>::new();
    let ptr1: *mut u8 = generator.any_in_bounds().ptr;
    let ptr2: *mut u8 = generator.any_in_bounds().ptr;
    // SAFETY: Both pointers have the same provenance.
    let distance = unsafe { ptr1.offset_from(ptr2) };
    assert!(distance >= -5 && distance <= 5)
}
