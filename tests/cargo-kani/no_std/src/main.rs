// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that Kani handles assert in no_std environment which
//! currently doesn't work:
//! https://github.com/model-checking/kani/issues/2187)

#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[kani::proof]
fn foo() {
    let x: i32 = kani::any();
    let y = 0;
    std::debug_assert!(x + y == x, "Message");
}

fn main() {}
