// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `compare_exchange` and `compare_exchange_weak` return `Err` when
// the expected value doesn't match the current value.

use std::sync::atomic::{AtomicU8, Ordering};

#[kani::proof]
fn check_compare_exchange_failure() {
    // Create an atomic with initial value 0
    let a = AtomicU8::new(0);

    // Try to compare_exchange with expected value 1 (but current is 0)
    // This should fail and return Err(0)
    let result = a.compare_exchange(1, 2, Ordering::SeqCst, Ordering::SeqCst);
    assert!(result == Err(0), "compare_exchange should return Err when expected doesn't match");

    // The value should remain unchanged
    assert!(
        a.load(Ordering::SeqCst) == 0,
        "value should remain unchanged after failed compare_exchange"
    );
}

#[kani::proof]
fn check_compare_exchange_weak_failure() {
    // Create an atomic with initial value 0
    let a = AtomicU8::new(0);

    // Try to compare_exchange_weak with expected value 1 (but current is 0)
    // This should fail and return Err(0)
    let result = a.compare_exchange_weak(1, 2, Ordering::SeqCst, Ordering::SeqCst);
    assert!(
        result == Err(0),
        "compare_exchange_weak should return Err when expected doesn't match"
    );

    // The value should remain unchanged
    assert!(
        a.load(Ordering::SeqCst) == 0,
        "value should remain unchanged after failed compare_exchange_weak"
    );
}
