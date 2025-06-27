// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This tests whether we take into account use-as statements (`use XXX as
//! YYY;`) referring to external code when resolving paths in `kani::stub`
//! attributes.

use other_crate::MyType as MyFavoriteType;
use other_crate::inner_mod::magic_number42 as forty_two;
use other_crate::magic_number13 as thirteen;

#[kani::proof]
#[kani::stub(zero, thirteen)]
#[kani::stub(one, forty_two)]
#[kani::stub(two, MyFavoriteType::magic_number101)]
fn harness() {
    assert_eq!(zero(), thirteen());
    assert_eq!(one(), forty_two());
    assert_eq!(two(), MyFavoriteType::magic_number101());
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
