// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(trivial_bounds)]
#![allow(unused, trivial_bounds)]

#[kani::proof]
fn test_trivial_bounds()
where
    i32: Iterator,
{
    for _ in 2i32 {}
}

fn main() {}
