// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A codegen cache implementation that is functionally equivalent to that of `impl_no_stats.rs`, but also
//! records cache hits, misses, and the amount of wall clock time spent on each for debugging analysis.

use std::time::Duration;

use crate::codegen_cprover_gotoc::codegen::cache::{
    CACHE, CacheEntry, CodegenCacheVal, impl_stats::stats::CacheTime,
};

#[macro_export]
macro_rules! implement_cache {
    ($name:tt -- $($(@$global:tt)? [$field_name:tt] $key:path => $val:path),+) => {
        #[derive(Default)]
        /// The unified codegen cache used to store codegen work.
        pub struct $name {
            $($field_name: (HashImpl<$key, $val>, impl_stats::stats::CacheStats),)*
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

        impl Drop for $name {
            fn drop(&mut self) {
                self.print_stats();
            }
        }

        impl $name {
            fn print_stats(&self) {
                tracing::debug!("\n***CACHE STATS***");
                let mut all_stats = Vec::new();
                $(
                    let name = std::any::type_name::<$val>();
                    let stats: &impl_stats::stats::CacheStats = &self.$field_name.1;
                    tracing::debug!("{name}: {:?}", stats);
                    all_stats.push(stats);
                )*
                let (hits, total) = impl_stats::stats::total_hits_and_queries(all_stats.into_iter());
                let hit_rate = hits as f64 / total as f64 * 100_f64;
                tracing::debug!("TOTAL: {hits} hits / {total} queries ({hit_rate:.2?}%)\n");
            }
        }

        // Implement [CodegenCacheVal] for all of the provided values, so that they can be accessed
        // in the cache. Since we are recording stats, implement that version of the trait.
        $(
            impl CodegenCacheVal for $val {
                type Key = $key;
                fn get_individual_cache(cache: &CodegenCache) -> &HashImpl<Self::Key, Self> {
                    &cache.$field_name.0
                }
                fn get_individual_cache_mut(cache: &mut CodegenCache) -> &mut HashImpl<Self::Key, Self> {
                    &mut cache.$field_name.0
                }
                fn get_individual_stats_mut(cache: &mut CodegenCache) -> &mut impl_stats::stats::CacheStats {
                    &mut cache.$field_name.1
                }
            }
        )*
    };
}

// Stores a result for the value, or the time wasted already on a miss...
pub struct StatsEntry<V: CodegenCacheVal>(Result<V, Duration>, V::Key);

pub fn cache_entry_impl<V: CodegenCacheVal>(key: V::Key) -> StatsEntry<V> {
    let start = std::time::Instant::now();
    let found_value = CACHE.with_borrow(|cache| V::get_individual_cache(cache).get(&key).cloned());
    let query_time = start.elapsed();
    match found_value {
        Some(hit_val) => {
            insert_cache_timing::<V>(CacheTime::Hit(query_time));
            StatsEntry(Ok(hit_val), key)
        }
        None => StatsEntry(Err(query_time), key),
    }
}

fn insert_cache_timing<V: CodegenCacheVal>(time: CacheTime) {
    CACHE.with_borrow_mut(|cache| {
        let stats = V::get_individual_stats_mut(cache);
        stats.add_time(time);
    })
}

impl<V: CodegenCacheVal> CacheEntry for StatsEntry<V> {
    type EntryVal = V;

    fn tweak<F: FnOnce(&mut V)>(mut self, f: F) -> Self {
        if let Ok(found_val) = &mut self.0 {
            f(found_val)
        }

        self
    }

    fn or_insert_with<F: FnOnce() -> V>(self, f: F) -> V {
        match self.0 {
            Ok(cached_val) => cached_val,
            Err(miss_time_already) => {
                let start = std::time::Instant::now();
                let calculated = f();
                let calc_time = start.elapsed();

                CACHE.with_borrow_mut(|cache| {
                    V::get_individual_cache_mut(cache).insert(self.1, calculated.clone())
                });
                let total_time = start.elapsed() + miss_time_already;

                insert_cache_timing::<V>(CacheTime::Miss { calc_time, total_time });

                calculated
            }
        }
    }
}

/// Utilites for recording cache statistics.
pub(crate) mod stats {
    use std::time::Duration;

    #[derive(Default, Clone)]
    pub(crate) struct CacheStats {
        /// The end to end time taken to return a result for each cache hit.
        hit_times: Vec<Duration>,
        /// The end to end time taken to return a result for each cache miss.
        miss_times: Vec<Duration>,
        /// The time taken to calculate a new entry's value on each cache miss
        /// (essentially the miss time minus the time spent inserting the new value into the cache).
        calc_times: Vec<Duration>,
    }

    impl CacheStats {
        /// The number of queries to this cache which hit.
        pub fn hits(&self) -> usize {
            self.hit_times.len()
        }

        /// The number of queries to this cache which missed.
        pub fn misses(&self) -> usize {
            // Since each miss should contain a calculations, the number of misses
            // should always be equal to the number of calculations.
            assert_eq!(self.calc_times.len(), self.miss_times.len());
            self.miss_times.len()
        }

        pub fn add_time(&mut self, time: CacheTime) {
            match time {
                CacheTime::Hit(time) => {
                    self.hit_times.push(time);
                }
                CacheTime::Miss { calc_time, total_time } => {
                    self.calc_times.push(calc_time);
                    self.miss_times.push(total_time);
                }
            }
        }
    }

    pub enum CacheTime {
        Hit(Duration),
        Miss { calc_time: Duration, total_time: Duration },
    }

    impl std::fmt::Debug for CacheStats {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let (hits, total) = (self.hits(), self.hits() + self.misses());
            let hit_rate = hits as f64 / total as f64 * 100_f64;
            write!(f, " {hits} hits / {total} queries ({hit_rate:.2?}%)")?;

            if self.hits() + self.misses() != 0 {
                write!(
                    f,
                    "- avg hit time: {:?}, avg miss time: {:?} (of which {:?} was calc)",
                    avg_duration(&self.hit_times),
                    avg_duration(&self.miss_times),
                    avg_duration(&self.calc_times)
                )?;
            }

            std::fmt::Result::Ok(())
        }
    }

    fn avg_duration(durations: &[Duration]) -> Duration {
        let len = durations.len() as u32;
        let sum: Duration = durations.iter().sum();
        sum / len
    }

    pub(crate) fn total_hits_and_queries<'a>(
        stats: impl Iterator<Item = &'a CacheStats>,
    ) -> (usize, usize) {
        stats.fold(<(usize, usize) as Default>::default(), |acc, stats| {
            (acc.0 + stats.hits(), acc.1 + stats.misses() + stats.hits())
        })
    }
}
