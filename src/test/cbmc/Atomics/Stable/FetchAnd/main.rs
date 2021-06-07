// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    // pub fn fetch_and(&self, val: bool, order: Ordering) -> bool
    // Performs a logical "and" operation on the current value and
    // the argument val, and sets the new value to the result.
    // Returns the previous value.
    let a1 = AtomicBool::new(true);
    let a2 = AtomicBool::new(true);
    let a3 = AtomicBool::new(true);
    let a4 = AtomicBool::new(true);
    let a5 = AtomicBool::new(true);

    // fetch_and is the stable version of atomic_and
    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#623-629
    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#2468-2481
    assert!(a1.fetch_and(false, Ordering::Acquire) == true);
    assert!(a2.fetch_and(false, Ordering::Release) == true);
    assert!(a3.fetch_and(false, Ordering::AcqRel) == true);
    assert!(a4.fetch_and(false, Ordering::Relaxed) == true);
    assert!(a5.fetch_and(false, Ordering::SeqCst) == true);

    assert!(a1.load(Ordering::SeqCst) == false);
    assert!(a2.load(Ordering::SeqCst) == false);
    assert!(a3.load(Ordering::SeqCst) == false);
    assert!(a4.load(Ordering::SeqCst) == false);
    assert!(a5.load(Ordering::SeqCst) == false);
}
