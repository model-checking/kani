// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the performance of pushing multiple non-det elements onto a
//! `BtreeSet`

use kani::cover;
use std::collections::BTreeSet;

#[kani::proof]
#[kani::unwind(3)]
#[kani::solver(cadical)]
fn insert_multi() {
    const N: usize = 2;
    let mut set: BTreeSet<i32> = BTreeSet::new();
    for _i in 0..N {
        set.insert(kani::any());
    }
    assert!(!set.is_empty());
    // all elements are the same
    cover!(set.len() == 1);
    // two unique elements
    cover!(set.len() == 2);
}

fn main() {}
