// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z mem-predicates
//! Check same allocation predicate.

extern crate kani;

use kani::mem::same_allocation;
use kani::{AllocationStatus, ArbitraryPointer, PointerGenerator};

#[kani::proof]
fn check_inbounds() {
    let mut generator = PointerGenerator::<100>::new();
    let ArbitraryPointer { ptr: ptr1, .. } = generator.any_in_bounds::<u8>();
    let ArbitraryPointer { ptr: ptr2, .. } = generator.any_in_bounds::<u8>();
    assert!(same_allocation(ptr1, ptr2));
}

#[kani::proof]
fn check_inbounds_other_alloc() {
    let mut generator1 = PointerGenerator::<100>::new();
    let mut generator2 = PointerGenerator::<100>::new();
    let ArbitraryPointer { ptr: ptr1, .. } = generator1.any_in_bounds::<u8>();
    let ArbitraryPointer { ptr: ptr2, .. } = generator2.any_in_bounds::<u8>();
    assert!(!same_allocation(ptr1, ptr2));
}

#[kani::proof]
fn check_dangling() {
    let mut generator = PointerGenerator::<100>::new();
    let ArbitraryPointer { ptr: ptr1, status: status1, .. } = generator.any_alloc_status::<u8>();
    let ArbitraryPointer { ptr: ptr2, status: status2, .. } = generator.any_alloc_status::<u8>();
    kani::assume(status1 == AllocationStatus::Dangling && status2 == AllocationStatus::InBounds);
    assert!(!same_allocation(ptr1, ptr2));
}

#[kani::proof]
fn check_one_dead() {
    let mut generator = PointerGenerator::<100>::new();
    let ArbitraryPointer { ptr: ptr1, status: status1, .. } = generator.any_alloc_status::<u8>();
    let ArbitraryPointer { ptr: ptr2, status: status2, .. } = generator.any_alloc_status::<u8>();
    kani::assume(status1 == AllocationStatus::DeadObject && status2 == AllocationStatus::InBounds);
    assert!(!same_allocation(ptr1, ptr2));
}

#[kani::proof]
fn check_dyn_alloc() {
    let mut generator1 = Box::new(PointerGenerator::<100>::new());
    let mut generator2 = Box::new(PointerGenerator::<100>::new());
    let ArbitraryPointer { ptr: ptr1a, .. } = generator1.any_in_bounds::<u8>();
    let ArbitraryPointer { ptr: ptr1b, .. } = generator1.any_in_bounds::<u8>();
    assert!(same_allocation(ptr1a, ptr1b));

    let ArbitraryPointer { ptr: ptr2, .. } = generator2.any_in_bounds::<u8>();
    assert!(!same_allocation(ptr1a, ptr2));
}
