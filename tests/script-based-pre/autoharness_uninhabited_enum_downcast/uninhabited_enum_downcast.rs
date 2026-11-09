// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(never_type)]
#[derive(Copy, Clone)]
pub enum E {
    A(!),
}
pub union U {
    u: (),
    e: E,
}

pub fn trigger() {
    let E::A(ref _a) = unsafe { &(&U { u: () }).e };
}

pub fn ok_fn(x: u8) -> u8 {
    x / 2
}
