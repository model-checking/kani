// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z mem-predicates
//! Check that pointers generated earlier from a `PointerGenerator` stay usable after generating
//! more pointers from it: the generator methods take `&self`, so several `ArbitraryPointer`s can
//! be alive at once, and a write through one pointer is visible through an overlapping one.
//!
//! With `&mut self` methods, the reborrow of the generator for each call invalidated the earlier
//! pointers under Stacked Borrows and Tree Borrows (Miri reports UB for the same program with
//! the generator's previous implementation).
extern crate kani;

use kani::PointerGenerator;

#[kani::proof]
fn check_two_live_pointers() {
    let generator = PointerGenerator::<8>::new();
    let first = generator.any_in_bounds::<u32>();
    let second = generator.any_in_bounds::<u32>();
    unsafe {
        first.ptr.write_unaligned(1);
        second.ptr.write_unaligned(2);
        if first.ptr == second.ptr {
            assert_eq!(first.ptr.read_unaligned(), 2);
        } else if first.ptr.addr().abs_diff(second.ptr.addr()) >= size_of::<u32>() {
            assert_eq!(first.ptr.read_unaligned(), 1);
        }
        kani::cover!(first.ptr == second.ptr, "same address");
        kani::cover!(first.ptr.addr() + size_of::<u32>() <= second.ptr.addr(), "disjoint");
    }
}
