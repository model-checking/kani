// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(generators, generator_trait)]

use std::ops::{Generator, GeneratorState};
use std::pin::Pin;

#[kani::proof]
#[kani::unwind(2)]
fn main() {
    let val: bool = kani::any();
    let mut generator = move || {
        let x = val;
        yield x;
        return !x;
    };

    let res = Pin::new(&mut generator).resume(());
    assert_eq!(res, GeneratorState::Yielded(val));
    let res = Pin::new(&mut generator).resume(());
    assert_eq!(res, GeneratorState::Complete(!val));
}
