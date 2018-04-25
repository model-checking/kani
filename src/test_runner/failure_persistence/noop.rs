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

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

/// Failure persistence option that loads and saves nothing at all.
#[derive(Debug, Default, PartialEq)]
struct NoopFailurePersistence;

impl FailurePersistence for NoopFailurePersistence {
    fn load_persisted_failures(&self, _source_file: Option<&'static str>) -> Vec<[u32; 4]> {
        Vec::new()
    }
    fn save_persisted_failure(
        &mut self,
        _source_file: Option<&'static str>,
        _seed: [u32; 4],
        _shrunken_value: &fmt::Debug,
    ) {
    }

    fn box_clone(&self) -> Box<FailurePersistence> {
        Box::new(NoopFailurePersistence)
    }

    fn eq(&self, other: &FailurePersistence) -> bool {
        other.as_any().downcast_ref::<Self>().map_or(false, |x| x == self)
    }

    fn as_any(&self) -> &Any { self }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_load_is_empty() {
        assert!(NoopFailurePersistence::default().load_persisted_failures(None).is_empty());
        assert!(NoopFailurePersistence::default().load_persisted_failures(Some("hi")).is_empty());
    }

    #[test]
    fn seeds_not_recoverable() {
        let mut p = NoopFailurePersistence::default();
        let seed = [0u32, 1, 2, 3];
        let key = Some("hi");
        p.save_persisted_failure(key, seed, &"");
        assert!(p.load_persisted_failures(key).is_empty());
        assert!(p.load_persisted_failures(None).is_empty());
        assert!(p.load_persisted_failures(Some("unrelated")).is_empty());
    }
}
