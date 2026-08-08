// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file tests whether we can generate a bounded BTreeMap/BTreeSet that has any possible size between 0-BOUND

#[kani::proof]
#[kani::unwind(5)]
fn check_btreemap() {
    // a larger bound causes this to take a long time, see bounded-arbitrary/hash
    const BOUND: usize = 1;
    let btree_map: std::collections::BTreeMap<u8, bool> = kani::bounded_any::<_, BOUND>();
    assert!(btree_map.len() <= BOUND);
    kani::cover!(btree_map.len() == 0);
    kani::cover!(btree_map.len() == 1);
}

#[kani::proof]
#[kani::unwind(5)]
fn check_btreeset() {
    // a larger bound causes this to take a long time, see bounded-arbitrary/hash
    const BOUND: usize = 1;
    let btree_set: std::collections::BTreeSet<u8> = kani::bounded_any::<_, BOUND>();
    assert!(btree_set.len() <= BOUND);
    kani::cover!(btree_set.len() == 0);
    kani::cover!(btree_set.len() == 1);
}
