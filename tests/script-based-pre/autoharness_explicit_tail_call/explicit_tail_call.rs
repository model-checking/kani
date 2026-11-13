// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(explicit_tail_calls)]
#![allow(incomplete_features)]

pub fn tail_call(x: u8) -> u8 {
    become ok_fn(x)
}

pub fn ok_fn(x: u8) -> u8 {
    x / 2
}
