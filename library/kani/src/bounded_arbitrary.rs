// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces implementations for some std containers.

use kani::{Arbitrary, BoundedArbitrary};

impl<K, V> BoundedArbitrary
    for std::collections::HashMap<K, V, std::hash::BuildHasherDefault<std::hash::DefaultHasher>>
where
    K: Arbitrary + std::cmp::Eq + std::hash::Hash,
    V: Arbitrary,
{
    fn bounded_any<const N: usize>() -> Self {
        let mut hash_map = std::collections::HashMap::default();
        for _ in 0..N {
            // this check seems to perform better than 0..kany::any_where(|l| *l <= N)
            if bool::any() {
                hash_map.insert(K::any(), V::any());
            }
        }
        hash_map
    }
}

impl<V> BoundedArbitrary
    for std::collections::HashSet<V, std::hash::BuildHasherDefault<std::hash::DefaultHasher>>
where
    V: Arbitrary + std::cmp::Eq + std::hash::Hash,
{
    fn bounded_any<const N: usize>() -> Self {
        let mut hash_set = std::collections::HashSet::default();
        for _ in 0..N {
            // this check seems to perform better than 0..kany::any_where(|l| *l <= N)
            if bool::any() {
                hash_set.insert(V::any());
            }
        }
        hash_set
    }
}
