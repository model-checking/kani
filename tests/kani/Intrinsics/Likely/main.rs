// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `likely` and `unlikely` return the value of the condition passed
// as an argument. These intrinsics hint the Rust compiler what branch in an
// `if`-`else` statement is more probable to be executed, allowing it to
// optimize code for the execution of one of the two branches:
// https://rust-lang.github.io/rfcs/1131-likely-intrinsic.html
// This aspect is not relevant for verification, which is why it is not modeled.

#![feature(core_intrinsics)]
use std::intrinsics::{likely, unlikely};

fn check_likely(x: i32, y: i32) {
    if likely(x != y) {
        assert!(x != y);
    } else {
        assert!(x == y);
    }
}

fn check_unlikely(x: i32, y: i32) {
    if unlikely(x == y) {
        assert!(x == y);
    } else {
        assert!(x != y);
    }
}

#[kani::proof]
fn check_likely_main() {
    let x = kani::any();
    let y = kani::any();
    check_likely(x, y);
}

#[kani::proof]
fn check_unlikely_main() {
    let x = kani::any();
    let y = kani::any();
    check_unlikely(x, y);
}
