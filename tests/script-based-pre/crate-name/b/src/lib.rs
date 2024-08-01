// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
extern crate a;

pub fn add_b(left: usize, right: usize) -> usize {
    a::add_a(left, right)
}
