// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::sync::atomic::{fence, Ordering};

fn main() {
    // pub fn fence(order: Ordering)
    // An atomic fence.
    // Depending on the specified order, a fence prevents the compiler
    // and CPU from reordering certain types of memory operations around it.
    // That creates synchronizes-with relationships between it and atomic
    // operations or fences in other threads.

    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#2662-2675
    fence(Ordering::Acquire);
    fence(Ordering::Release);
    fence(Ordering::AcqRel);
    fence(Ordering::SeqCst);

    // Nothing to assert
}
