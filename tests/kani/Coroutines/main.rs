// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This tests that coroutines work, even with a non-() resume type.

#![feature(coroutines, coroutine_trait)]
#![feature(stmt_expr_attributes)]

use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;

#[kani::proof]
#[kani::unwind(3)]
fn main() {
    let mut add_one = #[coroutine]
    |mut resume: u8| {
        loop {
            resume = yield resume.saturating_add(1);
        }
    };
    for _ in 0..2 {
        let val = kani::any();
        let res = Pin::new(&mut add_one).resume(val);
        assert_eq!(res, CoroutineState::Yielded(val.saturating_add(1)));
    }
}
