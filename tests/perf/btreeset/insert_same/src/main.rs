// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the performance of pushing the same element onto a `BTreeSet`
//! The test is from <https://github.com/model-checking/kani/issues/2022>
//! With CBMC's default solver (minisat), it takes ~517 seconds
//! With Kissat 3.0.0, it takes ~22 seconds
//! It started failing after <https://github.com/model-checking/kani/pull/3080>

use std::collections::BTreeSet;

#[kani::proof]
#[kani::unwind(3)]
#[kani::solver(minisat)]
fn main() {
    let mut set: BTreeSet<i32> = BTreeSet::new();
    let x = kani::any();
    set.insert(x);
    set.insert(x);
    assert!(set.len() == 1);
}
