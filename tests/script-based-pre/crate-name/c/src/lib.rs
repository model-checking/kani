// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
extern crate a;
extern crate b;

pub fn add_c(left: usize, right: usize) -> usize {
    b::add_b(left, right)
}
