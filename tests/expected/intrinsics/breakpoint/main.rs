// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `breakpoint` is supported (generates a `SKIP` statement)
// and that the assertion after `breakpoint` continues to be reachable
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    unsafe { std::intrinsics::breakpoint() };
    assert!(true);
}
