// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Copyright tokio Contributors
// origin: tokio-test/tests/ at commit b2ada60e701d5c9e6644cf8fc42a100774f8e23f

#![warn(rust_2018_idioms)]

use tokio::time::{Duration, Instant, sleep_until};
use tokio_test::block_on;

#[cfg(disabled)] // because epoll is missing
#[kani::proof]
#[kani::unwind(2)]
fn async_block() {
    assert_eq!(4, block_on(async { 4 }));
}

async fn five() -> u8 {
    5
}

#[cfg(disabled)] // because epoll is missing
#[kani::proof]
#[kani::unwind(2)]
fn async_fn() {
    assert_eq!(5, block_on(five()));
}

#[test]
fn test_sleep() {
    let deadline = Instant::now() + Duration::from_millis(100);

    block_on(async {
        sleep_until(deadline).await;
    });
}
