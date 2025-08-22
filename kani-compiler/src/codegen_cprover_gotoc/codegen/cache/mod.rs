// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module provides a 'unified' cache for Kani's codegen.
//!
//! The cache is 'unified' in the sense that it can cache multiple different pieces of our codegen,
//! but each can be queried and contained in a single struct.
//!
//! The core part of this implementation is the [implement_cache!] macro, which expands to the full
//! struct of the [CodegenCache], and all its needed implementation. In debug mode, this will use
//! the implementation from `impl_stats.rs`, which instruments all queries to capture cache statistics and
//! print them when the [CodegenCache] gets dropped after codegen. In release mode, it will instead use the
//! implementation from `impl_no_stats.rs`, which just has the core cache implementation.
//!
//! To interact with the cache elsewhere in Kani, just call [cache_entry] and pass in a key type from where
//! the cache is defined with the [implement_cache!] macro below. This will return a cache entry for the value type,
//! which you can then call [or_insert_with](CacheEntry::or_insert_with) on to get the cached value, or insert one if
//! none was found.
//!
//! If you wish to add another element to the cache, add another row to the [implement_cache!] macro call below,
//! and this will allow you to use [cache_entry] with that new key type.

use fxhash::FxHashMap;
use std::cell::RefCell;
use std::hash::Hash;

#[cfg(debug_assertions)]
pub mod impl_stats;
#[cfg(debug_assertions)]
pub use impl_stats::cache_entry_impl;

#[cfg(not(debug_assertions))]
pub mod impl_no_stats;
#[cfg(not(debug_assertions))]
pub use impl_no_stats::cache_entry_impl;

// This will automatically point to the `implement_cache` macro for whichever implementation
// module is currently cfg-ed in.
use crate::implement_cache;

thread_local! {
    /// The thread-local codegen cache. Since currently codegen is constrainted to be done
    /// in a single thread (since the compiler's [TyCtxt](rustc_middle::ty::TyCtxt) isn't `Send`),
    /// we only ever need the cache in that one thread.
    pub static CACHE: RefCell<CodegenCache> = RefCell::new(Default::default());
}

/// The hashmap implementation used to store data in the cache.
type HashImpl<K, V> = FxHashMap<K, V>;

// Define the actual cache implementation. This will expand to the struct defintion and
// implementation based on whether or not we want to record cache statistics.
//
// Each row of the macro represents the cache for a different type, whose syntax is:
// `@granularity [name_of_field_in_struct] KeyType => ValueType`
//
// See [clear_cache_field] below for how the `@granularity` annotation works.
implement_cache!(CodegenCache --
            [types] rustc_public::ty::Ty   => cbmc::goto_program::Type,
    @global [spans] rustc_public::ty::Span => cbmc::goto_program::Location
);

/// Get the cache entry for a specific key.
pub fn cache_entry<V: CodegenCacheVal>(key: V::Key) -> impl CacheEntry<EntryVal = V> {
    cache_entry_impl(key)
}

/// The trait for an entry in the cache, that provides the core API without us having to know how
/// the cache is implemented (namely whether stats are being recorded or not).
pub trait CacheEntry {
    /// The the of the value that this cache entry holds.
    type EntryVal: CodegenCacheVal;

    /// Applies `f` to modify the value found in the cache, if there was one.
    fn tweak<F: FnOnce(&mut Self::EntryVal)>(self, f: F) -> Self;

    /// Returns the cached value if there was one, or inserts a new value to the cache by calling `f`.
    fn or_insert_with<F: FnOnce() -> Self::EntryVal>(self, f: F) -> Self::EntryVal;
}

/// A type whose value can be stored in the codegen cache and retrived with a
/// specific corresponding [Key](CodegenCacheVal::Key) type.
pub trait CodegenCacheVal: Clone
where
    Self: Sized,
{
    type Key: Hash + Eq;

    /// Gets the underlying [HashImpl] used to cache this type in the unified [CodegenCache].
    fn get_individual_cache(cache: &CodegenCache) -> &HashImpl<Self::Key, Self>;
    /// Mutably gets the underlying [HashImpl] used to cache this type in the unified [CodegenCache].
    fn get_individual_cache_mut(cache: &mut CodegenCache) -> &mut HashImpl<Self::Key, Self>;

    #[cfg(debug_assertions)]
    /// Mutably gets the struct used to store statistics on cache performance for this type
    /// in the unified [CodegenCache].
    fn get_individual_stats_mut(cache: &mut CodegenCache) -> &mut impl_stats::stats::CacheStats;
}

#[macro_export]
/// Clears the cache field for a given type.
macro_rules! clear_cache_field {
    ( $cache:tt, $field_val:ty) => {
        // if no granularity is provided, default to clearing the field per-harness
        $crate::clear_cache_field!(per_harness $cache, $field_val);
    };
    (per_harness $cache:tt, $field_val:ty) => {
        <$field_val>::get_individual_cache_mut($cache).clear();
    };
    (global $cache:tt, $field_val:ty) => {
        /* global field, don't clear cache */
    };
}
