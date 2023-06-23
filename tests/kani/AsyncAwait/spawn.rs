// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// compile-flags: --edition 2018
// kani-flags: -Z async-lib

//! This file tests the executor and spawn infrastructure from the Kani library.

use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};

#[kani::proof]
#[kani::unwind(4)]
fn round_robin_schedule() {
    let x = Arc::new(AtomicI64::new(0)); // Surprisingly, Arc verified faster than Rc
    let x2 = x.clone();
    kani::block_on_with_spawn(
        async move {
            let x3 = x2.clone();
            kani::spawn(async move {
                assert_eq!(x3.load(Ordering::Relaxed), 0); // to check the order of the round-robin
                x3.fetch_add(1, Ordering::Relaxed);
            });
            kani::yield_now().await;
            assert_eq!(x2.load(Ordering::Relaxed), 1); // to check the order of the round-robin
            x2.fetch_add(1, Ordering::Relaxed);
        },
        kani::RoundRobin::default(),
    );
    assert_eq!(x.load(Ordering::Relaxed), 2);
}
