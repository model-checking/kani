// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces implementations for some std containers.

use kani::{Arbitrary, BoundedArbitrary};

impl<T: Arbitrary> BoundedArbitrary for Vec<T> {
    fn bounded_any<const N: usize>() -> Self {
        let real_length = kani::any_where(|&size| size <= N);
        let boxed_array: Box<[T; N]> = Box::new(kani::any());

        let mut vec = <[T]>::into_vec(boxed_array);

        // SAFETY: real length is less then or equal to N
        unsafe {
            vec.set_len(real_length);
        }

        kani::assume(vec.len() <= N);
        vec
    }
}

impl BoundedArbitrary for String {
    fn bounded_any<const N: usize>() -> Self {
        let bytes: [u8; N] = kani::any();

        let mut string = String::new();
        bytes.utf8_chunks().for_each(|chunk| string.push_str(chunk.valid()));

        kani::assume(string.len() <= N);
        string
    }
}

impl BoundedArbitrary for std::ffi::OsString {
    fn bounded_any<const N: usize>() -> Self {
        let bounded_string = String::bounded_any::<N>();
        bounded_string.into()
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
            hash_map.insert(K::any(), V::any());
        }
        hash_map
    }
}
