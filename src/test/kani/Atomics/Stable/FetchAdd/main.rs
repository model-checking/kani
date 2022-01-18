// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::sync::atomic::{AtomicIsize, Ordering};

fn main() {
    // pub fn fetch_add(&self, val: isize, order: Ordering) -> isize
    // Adds to the current value, returning the previous value.
    let a1 = AtomicIsize::new(0);
    let a2 = AtomicIsize::new(0);
    let a3 = AtomicIsize::new(0);
    let a4 = AtomicIsize::new(0);
    let a5 = AtomicIsize::new(0);

    // fetch_add is the stable version of atomic_add
    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#1717-1724
    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#2379-2392
    assert!(a1.fetch_add(1, Ordering::Acquire) == 0);
    assert!(a2.fetch_add(1, Ordering::Release) == 0);
    assert!(a3.fetch_add(1, Ordering::AcqRel) == 0);
    assert!(a4.fetch_add(1, Ordering::Relaxed) == 0);
    assert!(a5.fetch_add(1, Ordering::SeqCst) == 0);

    assert!(a1.load(Ordering::SeqCst) == 1);
    assert!(a2.load(Ordering::SeqCst) == 1);
    assert!(a3.load(Ordering::SeqCst) == 1);
    assert!(a4.load(Ordering::SeqCst) == 1);
    assert!(a5.load(Ordering::SeqCst) == 1);
}
