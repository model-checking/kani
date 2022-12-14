// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This tests whether we take into account `extern crate XXX as YYY;`
//! statements when resolving paths in `kani::stub` attributes.

extern crate other_crate as foo;

#[kani::proof]
#[kani::stub(zero, foo::magic_number13)]
#[kani::stub(one, foo::inner_mod::magic_number42)]
#[kani::stub(two, foo::MyType::magic_number101)]
fn harness() {
    assert_eq!(zero(), foo::magic_number13());
    assert_eq!(one(), foo::inner_mod::magic_number42());
    assert_eq!(two(), foo::MyType::magic_number101());
}

fn zero() -> u32 {
    0
}

fn one() -> u32 {
    1
}

fn two() -> u32 {
    2
}
