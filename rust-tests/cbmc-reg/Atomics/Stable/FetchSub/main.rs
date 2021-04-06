// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::sync::atomic::{AtomicIsize, Ordering};

fn main() {
    // pub fn fetch_sub(&self, val: isize, order: Ordering) -> isize
    // Subtracts from the current value, returning the previous value.
    let a1 = AtomicIsize::new(1);
    let a2 = AtomicIsize::new(1);
    let a3 = AtomicIsize::new(1);
    let a4 = AtomicIsize::new(1);
    let a5 = AtomicIsize::new(1);

    // fetch_sub is the stable version of atomic_sub
    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#1748-1755
    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#2395-2408
    assert!(a1.fetch_sub(1, Ordering::Acquire) == 1);
    assert!(a2.fetch_sub(1, Ordering::Release) == 1);
    assert!(a3.fetch_sub(1, Ordering::AcqRel) == 1);
    assert!(a4.fetch_sub(1, Ordering::Relaxed) == 1);
    assert!(a5.fetch_sub(1, Ordering::SeqCst) == 1);

    assert!(a1.load(Ordering::SeqCst) == 0);
    assert!(a2.load(Ordering::SeqCst) == 0);
    assert!(a3.load(Ordering::SeqCst) == 0);
    assert!(a4.load(Ordering::SeqCst) == 0);
    assert!(a5.load(Ordering::SeqCst) == 0);
}
