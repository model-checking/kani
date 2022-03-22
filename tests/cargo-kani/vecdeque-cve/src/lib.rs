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

use crate::cve::VecDeque as CveVecDeque;
use std::collections::VecDeque;

#[kani::proof]
pub fn cve_harness() {
    let mut q: CveVecDeque<i32> = CveVecDeque::<i32>::with_capacity(7);
    q.push_front(0);
    q.reserve(6);
    q.push_back(0);
}
