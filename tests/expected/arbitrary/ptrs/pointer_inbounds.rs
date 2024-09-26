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
    let mut generator = PointerGenerator::<char, 3>::new();
    let ptr = generator.any_in_bounds().ptr;
    kani::cover!(!kani::mem::can_read_unaligned(ptr), "Uninitialized");
    kani::cover!(kani::mem::can_read_unaligned(ptr), "Initialized");
    assert!(kani::mem::can_write_unaligned(ptr), "ValidWrite");
}

#[kani::proof]
fn check_inbounds_initialized() {
    let mut generator = PointerGenerator::<char, 3>::new();
    let arbitrary = generator.any_in_bounds();
    kani::assume(arbitrary.is_initialized);
    assert!(kani::mem::can_read_unaligned(arbitrary.ptr), "ValidRead");
}

#[kani::proof]
fn check_alignment() {
    let mut generator = PointerGenerator::<char, 3>::new();
    let ptr = generator.any_in_bounds().ptr;
    kani::cover!(kani::mem::can_write(ptr), "Aligned");
    kani::cover!(!kani::mem::can_write(ptr), "Unaligned");
}

#[kani::proof]
fn check_overlap() {
    let mut generator = PointerGenerator::<(u8, u32), 3>::new();
    let ptr_1 = generator.any_in_bounds().ptr as usize;
    let ptr_2 = generator.any_in_bounds().ptr as usize;
    kani::cover!(ptr_1 == ptr_2, "Same");
    kani::cover!(ptr_1 > ptr_2 && ptr_1 < ptr_2 + 8, "Overlap");
    kani::cover!(ptr_1 > ptr_2 + 8, "Greater");
    kani::cover!(ptr_2 > ptr_1 + 8, "Smaller");
}
