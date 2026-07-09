// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file tests whether we can generate a bounded BTreeMap/BTreeSet that has any possible size between 0-BOUND

#[kani::proof]
#[kani::unwind(5)]
fn check_btreemap() {
    const BOUND: usize = 2;
    let btree_map: std::collections::BTreeMap<u8, bool> = kani::bounded_any::<_, BOUND>();
    kani::cover!(btree_map.len() == 0);
    kani::cover!(btree_map.len() == 1);
    kani::cover!(btree_map.len() == 2);
}

#[kani::proof]
#[kani::unwind(5)]
fn check_btreeset() {
    const BOUND: usize = 2;
    let btree_set: std::collections::BTreeSet<u8> = kani::bounded_any::<_, BOUND>();
    kani::cover!(btree_set.len() == 0);
    kani::cover!(btree_set.len() == 1);
    kani::cover!(btree_set.len() == 2);
}
