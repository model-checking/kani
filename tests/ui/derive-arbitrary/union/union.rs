// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that there's a compilation error if user tries to derive Arbitrary for union.

#[derive(kani::Arbitrary)]
union Wrapper {
    b: bool,
    c: char,
}

#[kani::proof]
fn dead_harness() {
    panic!("This shouldn't compile");
}
