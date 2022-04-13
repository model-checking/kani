// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Example from https://github.com/rust-lang/rust/issues/44800 with a smaller queue size
#![feature(global_allocator, alloc_system, allocator_api)]
extern crate alloc_system;

use alloc_system::System;
use std::collections::VecDeque;

#[global_allocator]
static ALLOCATOR: System = System;

fn main() {
    let mut q = VecDeque::with_capacity(7);
    q.push_front(0);
    q.reserve(6);
    q.push_back(0);
}
