// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(bench_black_box)]
use std::hint::black_box;

#[kani::proof]
fn main() {
    // black_box is an identity function that limits compiler optimizations
    let a = 10;
    let b = black_box(a);
    assert!(a == b);
}
