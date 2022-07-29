// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{
    future::Future,
    pin::Pin,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

/// Polls the future in a loop until completion (a very simple executor)
///
/// This is intended as a drop-in replacement for `futures::block_on`, which Kani cannot handle.
pub fn block_on<F: Future>(mut fut: F) -> <F as Future>::Output {
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
    #[inline]
    unsafe fn clone_waker(_: *const ()) -> RawWaker {
        NOOP_RAW_WAKER
    }

    #[inline]
    unsafe fn noop(_: *const ()) {}

    RawWaker::new(std::ptr::null(), &RawWakerVTable::new(clone_waker, noop, noop, noop))
};
