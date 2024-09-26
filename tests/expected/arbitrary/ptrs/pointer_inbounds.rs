// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z mem-predicates
//! Check different cases for `PointerGenerator` for in-bounds pointers.
//! TODO: Enable initialization checks (`-Z uninit-checks`) once we add support to unions.
//! The current instrumentation does not work in the presence of MaybeUninit which we use
//! to implement PointerGenerator.
//! Kani will detect the usage of MaybeUninit and fail the verification.
extern crate kani;

use kani::PointerGenerator;

#[kani::proof]
fn check_inbounds() {
    let mut generator = kani::pointer_generator::<char, 3>();
    let ptr = generator.any_in_bounds::<char>().ptr;
    kani::cover!(!kani::mem::can_read_unaligned(ptr), "Uninitialized");
    kani::cover!(kani::mem::can_read_unaligned(ptr), "Initialized");
    assert!(kani::mem::can_write_unaligned(ptr), "ValidWrite");
}

#[kani::proof]
fn check_inbounds_initialized() {
    let mut generator = kani::pointer_generator::<char, 3>();
    let arbitrary = generator.any_in_bounds::<char>();
    kani::assume(arbitrary.is_initialized);
    assert!(kani::mem::can_read_unaligned(arbitrary.ptr), "ValidRead");
}

#[kani::proof]
fn check_alignment() {
    let mut generator = kani::pointer_generator::<char, 2>();
    let ptr: *mut char = generator.any_in_bounds().ptr;
    if ptr.is_aligned() {
        assert!(kani::mem::can_write(ptr), "Aligned");
    } else {
        assert!(!kani::mem::can_write(ptr), "Not aligned");
        assert!(kani::mem::can_write_unaligned(ptr), "Unaligned");
    }
}

#[kani::proof]
fn check_overlap() {
    let mut generator = kani::pointer_generator::<u16, 5>();
    let ptr_1 = generator.any_in_bounds::<u16>().ptr as usize;
    let ptr_2 = generator.any_in_bounds::<u16>().ptr as usize;
    kani::cover!(ptr_1 == ptr_2, "Same");
    kani::cover!(ptr_1 == ptr_2 + 1, "Overlap");
    kani::cover!(ptr_1 > ptr_2 + 4, "Greater");
    kani::cover!(ptr_2 > ptr_1 + 4, "Smaller");
}
