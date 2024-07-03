// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness main -Z stubbing
//
//! This tests that we raise an error if a path in a `kani::stub` attribute can
//! resolve to multiple functions.

mod mod1 {
    fn foo() {}
}

mod mod2 {
    fn foo() {}
}

use mod1::*;
use mod2::*;

fn stub() {}

#[kani::proof]
#[kani::stub(foo, stub)]
fn main() {
    assert!(false);
}
