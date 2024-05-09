// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(coroutines, coroutine_trait)]

use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;

#[kani::proof]
#[kani::unwind(2)]
fn main() {
    let val: bool = kani::any();
    let mut coroutine = move || {
        let x = val;
        yield x;
        return !x;
    };

    let res = Pin::new(&mut coroutine).resume(());
    assert_eq!(res, CoroutineState::Yielded(val));
    let res = Pin::new(&mut coroutine).resume(());
    assert_eq!(res, CoroutineState::Complete(!val));
}
