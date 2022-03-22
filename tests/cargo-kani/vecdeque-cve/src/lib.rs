// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(slice_range)]
#![feature(extend_one)]
#![feature(try_reserve_kind)]
#![feature(allocator_api)]
#![feature(dropck_eyepatch)]
#![feature(rustc_attrs)]
#![feature(core_intrinsics)]
#![feature(ptr_internals)]
#![feature(rustc_allow_const_fn_unstable)]

mod cve;
mod fixed;
mod raw_vec;

#[cfg(cve)]
use crate::cve::VecDeque;

#[cfg(not(cve))]
use crate::fixed::VecDeque;

/// Prove that reserving a min capacity that is less than current capacity is no-op.
#[kani::proof]
pub fn reserve_less_capacity_is_no_op() {
    let mut vec_deque = VecDeque::<i8>::new();
    let old_capacity = vec_deque.capacity();

    // Insert an element to empty VecDeque.
    let front = kani::any();
    vec_deque.push_front(front);

    // Change extra capacity to *any* value that is less than current capacity.
    let new_capacity: usize = kani::any();
    kani::assume(new_capacity < old_capacity);
    vec_deque.reserve(new_capacity);

    // Capacity should stay the same.
    assert_eq!(vec_deque.capacity(), old_capacity);
}

/// Trigger failure described in the CVE.
#[kani::proof]
pub fn reserve_less_capacity_cve() {
    use crate::cve::VecDeque;

    let mut vec_deque = VecDeque::<i8>::new();
    let old_capacity = vec_deque.capacity();

    // Insert an element to empty VecDeque.
    let front = kani::any();
    vec_deque.push_front(front);

    // Change extra capacity to *any* value that is less than current capacity.
    let new_capacity: usize = kani::any();
    kani::assume(new_capacity < old_capacity);
    vec_deque.reserve(new_capacity);

    // Capacity should stay the same.
    assert_eq!(vec_deque.capacity(), old_capacity);
}
