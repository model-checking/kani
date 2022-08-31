// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file tests a hand-written spawn infrastructure and executor.
//! This should be replaced with code from the Kani library as soon as the executor can get merged.

use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};

include!("scheduler.rs");

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
