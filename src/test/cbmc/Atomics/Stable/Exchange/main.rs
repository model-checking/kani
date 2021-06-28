// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    let a1 = AtomicBool::new(true);
    let a2 = AtomicBool::new(true);
    let a3 = AtomicBool::new(true);
    let a4 = AtomicBool::new(true);
    let a5 = AtomicBool::new(true);

    // swap is the stable version of atomic_xchg
    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#435
    assert!(a1.swap(false, Ordering::Acquire) == true);
    assert!(a2.swap(false, Ordering::AcqRel) == true);
    assert!(a3.swap(false, Ordering::Relaxed) == true);
    assert!(a4.swap(false, Ordering::Release) == true);
    assert!(a5.swap(false, Ordering::SeqCst) == true);

    assert!(a1.load(Ordering::Relaxed) == false);
    assert!(a2.load(Ordering::Relaxed) == false);
    assert!(a3.load(Ordering::Relaxed) == false);
    assert!(a4.load(Ordering::Relaxed) == false);
    assert!(a5.load(Ordering::Relaxed) == false);
}
