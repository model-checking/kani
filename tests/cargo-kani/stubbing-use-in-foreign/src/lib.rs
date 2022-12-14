// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This tests whether we take into account use statements (`use XXX;`, `use XXX
//! as YYY`, and `use XXX::*`) that occur in external code when resolving paths
//! in `kani::stub` attributes.

use other_crate::*;

#[kani::proof]
#[kani::stub(zero, magic_number13)]
#[kani::stub(one, inner_mod::magic_number42)]
#[kani::stub(two, MyType::magic_number101)]
fn harness() {
    assert_eq!(zero(), magic_number13());
    assert_eq!(one(), inner_mod::magic_number42());
    assert_eq!(two(), MyType::magic_number101());
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
