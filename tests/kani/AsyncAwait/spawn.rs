// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// compile-flags: --edition 2018

//! This file tests the executor and spawn infrastructure from the Kani library.

use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};

#[kani::proof]
#[kani::unwind(4)]
fn deterministic_schedule() {
    let x = Arc::new(AtomicI64::new(0)); // Surprisingly, Arc verified faster than Rc
    let x2 = x.clone();
    spawnable_block_on(
        async move {
            let x3 = x2.clone();
            spawn(async move {
                x3.fetch_add(1, Ordering::Relaxed);
            });
            yield_now().await;
            x2.fetch_add(1, Ordering::Relaxed);
        },
        RoundRobin::default(),
    );
    assert_eq!(x.load(Ordering::Relaxed), 2);
}

#[kani::proof]
#[kani::unwind(4)]
fn nondeterministic_schedule() {
    let x = Arc::new(AtomicI64::new(0)); // Surprisingly, Arc verified faster than Rc
    let x2 = x.clone();
    spawnable_block_on(
        async move {
            let x3 = x2.clone();
            spawn(async move {
                x3.fetch_add(1, Ordering::Relaxed);
            });
            yield_now().await;
            x2.fetch_add(1, Ordering::Relaxed);
        },
        NondetFairScheduling::new(2),
    );
    assert_eq!(x.load(Ordering::Relaxed), 2);
}
