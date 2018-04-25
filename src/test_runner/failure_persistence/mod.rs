//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::fmt;
use core::any::Any;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::boxed::Box;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

#[cfg(feature = "std")]
mod file;
mod map;
mod noop;

#[cfg(feature = "std")]
pub use self::file::*;
pub use self::map::*;
pub use self::noop::*;

/// Provides external persistence for historical test failures by storing seeds.
pub trait FailurePersistence: Send + Sync + fmt::Debug  {
    /// Supply seeds associated with the given `source_file` that may be
    /// used by a `TestRunner`'s random number generator in order to
    /// consistently recreate a previously-failing `Strategy`-provided value.
    fn load_persisted_failures(&self, source_file: Option<&'static str>) -> Vec<[u32; 4]>;

    /// Store a new failure-generating seed associated with the given `source_file`.
    fn save_persisted_failure(
        &mut self,
        source_file: Option<&'static str>,
        seed: [u32; 4],
        shrunken_value: &fmt::Debug,
    );

    /// Delegate method for producing a trait object usable with `Clone`
    fn box_clone(&self) -> Box<FailurePersistence>;

    /// Equality testing delegate required due to constraints of trait objects.
    fn eq(&self, other: &FailurePersistence) -> bool;

    /// Assistant method for trait object comparison.
    fn as_any(&self) -> &Any;
}

impl<'a, 'b> PartialEq<FailurePersistence+'b> for FailurePersistence+'a {
    fn eq(&self, other: &(FailurePersistence+'b)) -> bool {
        FailurePersistence::eq(self, other)
    }
}

impl Clone for Box<FailurePersistence> {
    fn clone(&self) -> Box<FailurePersistence> {
        self.box_clone()
    }
}
