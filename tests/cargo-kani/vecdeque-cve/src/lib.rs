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
mod raw_vec;

use crate::cve::VecDeque;

// Bound proof to a maximum length.
const MAX_LENGTH: usize = 100;

#[kani::proof]
pub fn cve_harness() {
    // Insert element to empty VecDeque.
    let mut q: VecDeque<i32> = VecDeque::<i32>::new();
    let front = kani::any();
    q.push_front(front);

    // Change extra capacity to any value between 0 and MAX_LENGTH.
    let new_len: usize = kani::any();
    kani::assume(new_len <= MAX_LENGTH);
    q.reserve(6);

    // Push element to the back.
    let back = kani::any();
    q.push_back(back);

    // Assert front and back have the expected value.
    assert_eq!(front, *q.front().unwrap());
    assert_eq!(back, *q.back().unwrap());
}
