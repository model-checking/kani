// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains functions to work with futures (and async/.await) in Kani.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

/// A very simple executor: it polls the future in a busy loop until completion
///
/// This is intended as a drop-in replacement for `futures::block_on`, which Kani cannot handle.
/// Whereas a clever executor like `block_on` in `futures` or `tokio` would interact with the OS scheduler
/// to be woken up when a resource becomes available, this is not supported by Kani.
/// As a consequence, this function completely ignores the waker infrastructure and just polls the given future in a busy loop.
pub fn block_on<T>(mut fut: impl Future<Output = T>) -> T {
    let waker = unsafe { Waker::from_raw(NOOP_RAW_WAKER) };
    let cx = &mut Context::from_waker(&waker);
    // SAFETY: we shadow the original binding, so it cannot be accessed again for the rest of the scope.
    // This is the same as what the pin_mut! macro in the futures crate does.
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
    #[inline]
    unsafe fn clone_waker(_: *const ()) -> RawWaker {
        NOOP_RAW_WAKER
    }

    #[inline]
    unsafe fn noop(_: *const ()) {}

    RawWaker::new(std::ptr::null(), &RawWakerVTable::new(clone_waker, noop, noop, noop))
};
