// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// compile-flags: --edition 2018

// Tests that the language constructs `async { ... }` blocks, `async fn`, and `.await` work correctly.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

fn main() {}

#[kani::proof]
async fn test_async_proof_harness() {
    let async_block_result = async { 42 }.await;
    let async_fn_result = async_fn().await;
    assert_eq!(async_block_result, async_fn_result);
}

#[kani::proof]
pub async fn test_async_proof_harness_pub() {
    let async_block_result = async { 42 }.await;
    let async_fn_result = async_fn().await;
    assert_eq!(async_block_result, async_fn_result);
}

#[kani::proof]
fn test_async_await() {
    // Test using the `block_on` implementation in Kani's library
    kani::block_on(async {
        let async_block_result = async { 42 }.await;
        let async_fn_result = async_fn().await;
        assert_eq!(async_block_result, async_fn_result);
    })
}

#[kani::proof]
#[kani::unwind(2)]
fn test_async_await_manually() {
    // Test using the manual `block_on` implementation
    block_on(async {
        let async_block_result = async { 42 }.await;
        let async_fn_result = async_fn().await;
        assert_eq!(async_block_result, async_fn_result);
    })
}

pub async fn async_fn() -> i32 {
    42
}

/// A very simple executor that just polls the future in a loop
pub fn block_on<T>(mut fut: impl Future<Output = T>) -> T {
    let waker = unsafe { Waker::from_raw(NOOP_RAW_WAKER) };
    let cx = &mut Context::from_waker(&waker);
    let mut fut = unsafe { Pin::new_unchecked(&mut fut) };
    loop {
        match fut.as_mut().poll(cx) {
            std::task::Poll::Ready(res) => return res,
            std::task::Poll::Pending => continue,
        }
    }
}

/// A dummy waker, which is needed to call [`Future::poll`]
const NOOP_RAW_WAKER: RawWaker = {
    unsafe fn clone_waker(_: *const ()) -> RawWaker {
        NOOP_RAW_WAKER
    }
    unsafe fn noop(_: *const ()) {}
    RawWaker::new(std::ptr::null(), &RawWakerVTable::new(clone_waker, noop, noop, noop))
};
