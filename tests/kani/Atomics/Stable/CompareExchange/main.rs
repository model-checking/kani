// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    // pub fn compare_exchange(
    //     &self,
    //     current: bool,
    //     new: bool,
    //     success: Ordering,
    //     failure: Ordering
    // ) -> Result<bool, bool>
    // Stores a value into the bool if the current value is the same
    // as the current value.
    // The return value is a result indicating whether the new value
    // was written and containing the previous value. On success this
    // value is guaranteed to be equal to current.
    let a1 = AtomicBool::new(true);
    let a2 = AtomicBool::new(true);
    let a3 = AtomicBool::new(true);
    let a4 = AtomicBool::new(true);
    let a5 = AtomicBool::new(true);
    let a6 = AtomicBool::new(true);
    let a7 = AtomicBool::new(true);
    let a8 = AtomicBool::new(true);
    let a9 = AtomicBool::new(true);

    // compare_exchange is the stable version of atomic_cxchg
    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#1094-1115
    // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#2410-2437
    // Combinations other than the ones included here result in panic
    assert!(a1.compare_exchange(true, false, Ordering::Acquire, Ordering::Acquire) == Ok(true));
    assert!(a2.compare_exchange(true, false, Ordering::Release, Ordering::Relaxed) == Ok(true));
    assert!(a3.compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire) == Ok(true));
    assert!(a4.compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed) == Ok(true));
    assert!(a5.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst) == Ok(true));
    assert!(a6.compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed) == Ok(true));
    assert!(a7.compare_exchange(true, false, Ordering::AcqRel, Ordering::Relaxed) == Ok(true));
    assert!(a8.compare_exchange(true, false, Ordering::SeqCst, Ordering::Relaxed) == Ok(true));
    assert!(a9.compare_exchange(true, false, Ordering::SeqCst, Ordering::Acquire) == Ok(true));

    assert!(a1.load(Ordering::Relaxed) == false);
    assert!(a2.load(Ordering::Relaxed) == false);
    assert!(a3.load(Ordering::Relaxed) == false);
    assert!(a4.load(Ordering::Relaxed) == false);
    assert!(a5.load(Ordering::Relaxed) == false);
    assert!(a6.load(Ordering::Relaxed) == false);
    assert!(a7.load(Ordering::Relaxed) == false);
    assert!(a8.load(Ordering::Relaxed) == false);
    assert!(a9.load(Ordering::Relaxed) == false);
}
