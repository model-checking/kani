// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// compile-flags: --edition 2018

use std::{
    future::Future,
    pin::Pin,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

#[kani::proof]
#[kani::unwind(10)]
fn main() {
    poll_loop(async {
        let async_block_result = async { 42 }.await;
        let async_fn_result = async_fn().await;
        assert_eq!(async_block_result, async_fn_result);
    })
}

pub async fn async_fn() -> i32 {
    42
}

/// A very simple executor that just polls the future in a loop
pub fn poll_loop<F: Future>(mut fut: F) -> <F as Future>::Output {
    let waker = unsafe { Waker::from_raw(NOOP_RAW_WAKER) };
    let cx = &mut Context::from_waker(&waker);
    loop {
        let pinned = unsafe { Pin::new_unchecked(&mut fut) };
        match pinned.poll(cx) {
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
    unsafe fn wake(_: *const ()) {}
    unsafe fn wake_by_ref(_: *const ()) {}
    unsafe fn drop_waker(_: *const ()) {}
    RawWaker::new(
        std::ptr::null(),
        &RawWakerVTable::new(clone_waker, wake, wake_by_ref, drop_waker),
    )
};
