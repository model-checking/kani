// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file tests whether we can generate a bounded Hashmap/Hashset that has any possible size between 0-BOUND

#[kani::proof]
#[kani::unwind(5)]
fn check_hashmap() {
    // A larger bound causes this to take a long time
    const BOUND: usize = 1;
    let hash_map: std::collections::HashMap<u8, bool, _> = kani::bounded_any::<_, BOUND>();
    kani::cover!(hash_map.len() == 0);
    kani::cover!(hash_map.len() == 1);
}

#[kani::proof]
#[kani::unwind(5)]
fn check_hashset() {
    const BOUND: usize = 1;
    let hash_set: std::collections::HashSet<u8, _> = kani::bounded_any::<_, BOUND>();
    kani::cover!(hash_set.len() == 0);
    kani::cover!(hash_set.len() == 1);
}
