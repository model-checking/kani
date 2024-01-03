// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness main
//
//! This tests whether we abort if a stub is specified for a harness but stubbing is not enabled.

fn foo() -> u32 {
    0
}

fn bar() -> u32 {
    42
}

#[kani::proof]
#[kani::stub(foo, bar)]
fn main() {
    assert_eq!(foo(), 42);
}
