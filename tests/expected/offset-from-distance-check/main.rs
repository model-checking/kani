// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z mem-predicates
//! Check that Kani detects UB for offset_from when the distance between the pointers is not a multiple of size_of::<T>
extern crate kani;

#[kani::proof]
fn check_offset_from_distance_ub() {
    let mut generator = kani::pointer_generator::<u16, 5>();
    let ptr_1 = generator.any_in_bounds::<u16>().ptr;
    let ptr_2 = generator.any_in_bounds::<u16>().ptr;

    // offset_from is only safe if the distance between the pointers in bytes is a multiple of size_of::<T>,
    // which holds if either both ptr_1 and ptr_2 are aligned or neither are.
    // However, any_in_bounds makes no guarantees about alignment, so the above condition may not hold and verification should fail.
    unsafe { ptr_1.offset_from(ptr_2) };
}
