// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    // pub fn fetch_or(&self, val: bool, order: Ordering) -> bool
    // Performs a logical "or" operation on the current value and
    // the argument val, and sets the new value to the result.
    // Returns the previous value.
    let a1 = AtomicBool::new(true);
    let a2 = AtomicBool::new(true);
    let a3 = AtomicBool::new(true);
    let a4 = AtomicBool::new(true);
    let a5 = AtomicBool::new(true);

    // fetch_or is the stable version of atomic_or
    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#715-721
    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#2498-2511
    assert!(a1.fetch_or(false, Ordering::Acquire) == true);
    assert!(a2.fetch_or(false, Ordering::Release) == true);
    assert!(a3.fetch_or(false, Ordering::AcqRel) == true);
    assert!(a4.fetch_or(false, Ordering::Relaxed) == true);
    assert!(a5.fetch_or(false, Ordering::SeqCst) == true);

    assert!(a1.load(Ordering::SeqCst) == true);
    assert!(a2.load(Ordering::SeqCst) == true);
    assert!(a3.load(Ordering::SeqCst) == true);
    assert!(a4.load(Ordering::SeqCst) == true);
    assert!(a5.load(Ordering::SeqCst) == true);
}
