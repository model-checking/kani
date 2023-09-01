// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness harness -Z stubbing
//
//! Tests to make sure that, when we are resolving paths in `kani::stub`
//! attributes, we prioritize those that do not come from glob imports.

use my_mod::*;
use my_other_mod::other_magic_number;

mod my_mod {
    pub fn magic_number() -> u32 {
        13
    }

    pub fn other_magic_number() -> u32 {
        101
    }
}

mod my_other_mod {
    pub fn other_magic_number() -> u32 {
        102
    }
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

fn magic_number() -> u32 {
    42
}

#[kani::proof]
#[kani::stub(zero, magic_number)]
#[kani::stub(one, crate::magic_number)]
#[kani::stub(two, other_magic_number)]
fn harness() {
    assert_eq!(zero(), magic_number());
    assert_eq!(zero(), 42);
    assert_eq!(one(), crate::magic_number());
    assert_eq!(one(), 42);
    assert_eq!(two(), other_magic_number());
    assert_eq!(two(), 102);
}
