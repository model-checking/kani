//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::any::Any;
use core::fmt;
use test_runner::failure_persistence::FailurePersistence;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::boxed::Box;

#[cfg(all(feature = "alloc", not(feature="std")))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

#[cfg(all(feature = "alloc", not(feature="std")))]
use alloc::{BTreeMap, BTreeSet};
#[cfg(feature = "std")]
use std::collections::{BTreeMap, BTreeSet};

/// Failure persistence option that loads and saves seeds in memory
/// on the heap.
#[derive(Clone, Debug, PartialEq)]
pub struct MapFailurePersistence {
    /// Backing map, keyed by source_file.
    pub map: BTreeMap<&'static str, BTreeSet<[u32;4]>>
}

impl FailurePersistence for MapFailurePersistence {
    fn load_persisted_failures(&self, source_file: Option<&'static str>) -> Vec<[u32; 4]> {
        source_file
            .and_then(|source| self.map.get(source))
            .map(|seeds| seeds.iter().cloned().collect::<Vec<_>>())
            .unwrap_or(Vec::new())
    }
    fn save_persisted_failure(
        &mut self,
        source_file: Option<&'static str>,
        seed: [u32; 4],
        _shrunken_value: &fmt::Debug,
    ) {
        let s = match source_file {
            Some(sf) => sf,
            None => return
        };
        let set = self.map.entry(s).or_insert(BTreeSet::new());
        set.insert(seed);
    }

    fn box_clone(&self) -> Box<FailurePersistence> {
        Box::new(self.clone())
    }

    fn eq(&self, other: &FailurePersistence) -> bool {
        other.as_any().downcast_ref::<Self>().map_or(false, |x| x == self)
    }

    fn as_any(&self) -> &Any { self }
}

impl Default for MapFailurePersistence {
    fn default() -> Self {
        MapFailurePersistence { map: BTreeMap::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_map_is_empty() {
        assert!(MapFailurePersistence::default().load_persisted_failures(Some("hi")).is_empty())
    }

    #[test]
    fn seeds_recoverable() {
        let mut p = MapFailurePersistence::default();
        let seed = [0u32, 1, 2, 3];
        let key = Some("hi");
        p.save_persisted_failure(key, seed, &"");
        let restored = p.load_persisted_failures(key);
        assert_eq!(1, restored.len());
        assert_eq!(seed, *restored.first().unwrap());

        assert!(p.load_persisted_failures(None).is_empty());
        assert!(p.load_persisted_failures(Some("unrelated")).is_empty());
    }

    #[test]
    fn seeds_deduplicated() {
        let mut p = MapFailurePersistence::default();
        let seed = [0u32, 1, 2, 3];
        let key = Some("hi");
        p.save_persisted_failure(key, seed, &"");
        p.save_persisted_failure(key, seed, &"");
        let restored = p.load_persisted_failures(key);
        assert_eq!(1, restored.len());
    }
}
