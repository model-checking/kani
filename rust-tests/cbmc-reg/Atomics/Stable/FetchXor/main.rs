// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    // pub fn fetch_xor(&self, val: bool, order: Ordering) -> bool
    // Performs a bitwise "xor" operation on the current value and
    // the argument val, and sets the new value to the result.
    // Returns the previous value.
    let a1 = AtomicBool::new(true);
    let a2 = AtomicBool::new(true);
    let a3 = AtomicBool::new(true);
    let a4 = AtomicBool::new(true);
    let a5 = AtomicBool::new(true);

    // fetch_xor is the stable version of atomic_xor
    assert!(a1.fetch_xor(true, Ordering::Acquire) == true);
    assert!(a2.fetch_xor(true, Ordering::Release) == true);
    assert!(a3.fetch_xor(true, Ordering::AcqRel) == true);
    assert!(a4.fetch_xor(true, Ordering::Relaxed) == true);
    assert!(a5.fetch_xor(true, Ordering::SeqCst) == true);

    assert!(a1.load(Ordering::SeqCst) == false);
    assert!(a2.load(Ordering::SeqCst) == false);
    assert!(a3.load(Ordering::SeqCst) == false);
    assert!(a4.load(Ordering::SeqCst) == false);
    assert!(a5.load(Ordering::SeqCst) == false);
}
