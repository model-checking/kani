// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --enable-unstable --enable-stubbing
//! Checks that the `kani::stub` attribute is accepted

fn foo() {
    unreachable!();
}

fn bar() {}

#[kani::proof]
#[kani::stub(foo, bar)]
fn main() {
    foo();
}
