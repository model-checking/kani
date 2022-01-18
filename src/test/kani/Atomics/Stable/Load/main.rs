// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    // pub fn load(&self, order: Ordering) -> bool
    // Loads a value from the bool.
    // load takes an Ordering argument which describes the memory ordering
    // of this operation.
    let a = AtomicBool::new(true);

    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#2349-2361
    assert!(a.load(Ordering::Acquire) == true);
    assert!(a.load(Ordering::Relaxed) == true);
    assert!(a.load(Ordering::SeqCst) == true);
}
