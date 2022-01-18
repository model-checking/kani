// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    // ppub fn store(&self, val: bool, order: Ordering)
    // Stores a value into the bool.
    // store takes an Ordering argument which describes the memory ordering
    // of this operation.
    let a1 = AtomicBool::new(true);
    let a2 = AtomicBool::new(true);
    let a3 = AtomicBool::new(true);

    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#2335-2347
    a1.store(false, Ordering::Release);
    a2.store(false, Ordering::Relaxed);
    a3.store(false, Ordering::SeqCst);

    assert!(a1.load(Ordering::SeqCst) == false);
    assert!(a2.load(Ordering::SeqCst) == false);
    assert!(a3.load(Ordering::SeqCst) == false);
}
