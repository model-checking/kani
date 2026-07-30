// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A core implementation of a unified codegen cache.

use crate::codegen_cprover_gotoc::codegen::cache::{CACHE, CacheEntry, CodegenCacheVal};

#[macro_export]
macro_rules! implement_cache {
    ($name:tt -- $($(@$global:tt)? [$field_name:tt] $key:path => $val:path),+) => {
        #[derive(Default)]
        /// The unified codegen cache used to store codegen work.
        pub struct $name {
            $($field_name: HashImpl<$key, $val>,)*
        }

        /// Reset the thread local [CodegenCache] between harnesses.
        /// Fields marked with `@global` won't be cleared.
        pub fn clear_codegen_cache() {
            CACHE.with_borrow_mut(|cache|{
                $(
                    $crate::clear_cache_field!($($global)? cache, $val);
                )*
            })
        }

        // Implement [CodegenCacheVal] for all of the provided cache values, so that they can be accessed
        // in the cache.
        $(
            impl CodegenCacheVal for $val {
                type Key = $key;
                fn get_individual_cache(cache: &CodegenCache) -> &HashImpl<Self::Key, Self> {
                    &cache.$field_name
                }
                fn get_individual_cache_mut(cache: &mut CodegenCache) -> &mut HashImpl<Self::Key, Self> {
                    &mut cache.$field_name
                }
            }
        )*
    };
}

pub struct NoStatsEntry<V: CodegenCacheVal>(Option<V>, V::Key);

pub fn cache_entry_impl<V: CodegenCacheVal>(key: V::Key) -> NoStatsEntry<V> {
    let found_value = CACHE.with_borrow(|cache| V::get_individual_cache(cache).get(&key).cloned());
    NoStatsEntry(found_value, key)
}

impl<V: CodegenCacheVal> CacheEntry for NoStatsEntry<V> {
    type EntryVal = V;

    fn tweak<F: FnOnce(&mut V)>(mut self, f: F) -> Self {
        if let Some(found_val) = &mut self.0 {
            f(found_val)
        }

        self
    }

    fn or_insert_with<F: FnOnce() -> V>(self, f: F) -> V {
        match self.0 {
            Some(cached) => cached,
            None => {
                let calculated = f();
                CACHE.with_borrow_mut(|cache| {
                    V::get_individual_cache_mut(cache).insert(self.1, calculated.clone())
                });
                calculated
            }
        }
    }
}
