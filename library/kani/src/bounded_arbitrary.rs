// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces implementations for some std containers.

use kani::{Arbitrary, BoundedArbitrary};

// This implementation overlaps with `kani::any_vec` in `kani/library/kani/src/vec.rs`.
// This issue `https://github.com/model-checking/kani/issues/4027` tracks deprecating
// `kani::any_vec` in favor of this implementation.
impl<T: Arbitrary> BoundedArbitrary for Vec<T> {
    fn bounded_any<const N: usize>() -> Self {
        let real_length = kani::any_where(|&size| size <= N);
        let array: [T; N] = kani::any();
        let mut vec = Vec::from(array);
        vec.truncate(real_length);
        vec
    }
}

impl BoundedArbitrary for String {
    fn bounded_any<const N: usize>() -> Self {
        let bytes: [u8; N] = kani::any();

        if let Some(s) = bytes.utf8_chunks().next() { s.valid().into() } else { String::new() }
    }
}

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
