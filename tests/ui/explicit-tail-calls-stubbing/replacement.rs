// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Stub validation and the stub pass load the replacement function's body. A `become` there is
//! reported as unsupported instead of panicking inside `rustc_public`.
//! <https://github.com/model-checking/kani/issues/5131>

#![feature(explicit_tail_calls)]
#![allow(incomplete_features)]

fn g(x: u32) -> u32 {
    x + 1
}

fn original(x: u32) -> u32 {
    x + 1
}

fn replacement(x: u32) -> u32 {
    become g(x)
}

#[kani::proof]
#[kani::stub(original, replacement)]
fn check_stubbed_replacement() {
    assert_eq!(original(1), 2);
}
