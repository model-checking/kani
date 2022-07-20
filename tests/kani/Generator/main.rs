// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This tests that generators work, even with a non-() resume type.

#![feature(generators, generator_trait)]

use std::ops::{Generator, GeneratorState};
use std::pin::Pin;

#[kani::proof]
fn main() {
    let mut add_one = |mut resume: u8| {
        loop {
            resume = yield resume.saturating_add(1);
        }
    };
    for _ in 0..2 {
        let val = kani::any();
        let res = Pin::new(&mut add_one).resume(val);
        assert_eq!(res, GeneratorState::Yielded(val.saturating_add(1)));
    }
}
