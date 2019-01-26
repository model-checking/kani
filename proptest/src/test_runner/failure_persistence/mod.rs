//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::any::Any;
use crate::std_facade::{fmt, Box, Vec};

#[cfg(feature = "std")]
mod file;
mod map;
mod noop;

#[cfg(feature = "std")]
pub use self::file::*;
pub use self::map::*;
pub use self::noop::*;

use crate::test_runner::Seed;

/// Provides external persistence for historical test failures by storing seeds.
pub trait FailurePersistence: Send + Sync + fmt::Debug  {
    /// Supply seeds associated with the given `source_file` that may be used
    /// by a `TestRunner`'s random number generator in order to consistently
    /// recreate a previously-failing `Strategy`-provided value.
    fn load_persisted_failures(&self, source_file: Option<&'static str>)
                               -> Vec<Seed>;

    /// Store a new failure-generating seed associated with the given `source_file`.
    fn save_persisted_failure(
        &mut self,
        source_file: Option<&'static str>,
        seed: Seed,
        shrunken_value: &dyn fmt::Debug,
    );

    /// Delegate method for producing a trait object usable with `Clone`
    fn box_clone(&self) -> Box<dyn FailurePersistence>;

    /// Equality testing delegate required due to constraints of trait objects.
    fn eq(&self, other: &dyn FailurePersistence) -> bool;

    /// Assistant method for trait object comparison.
    fn as_any(&self) -> &dyn Any;
}

impl<'a, 'b> PartialEq<dyn FailurePersistence + 'b>
for dyn FailurePersistence + 'a {
    fn eq(&self, other: &(dyn FailurePersistence + 'b)) -> bool {
        FailurePersistence::eq(self, other)
    }
}

impl Clone for Box<dyn FailurePersistence> {
    fn clone(&self) -> Box<dyn FailurePersistence> {
        self.box_clone()
    }
}

#[cfg(test)]
mod tests {
    pub const INC_SEED: [u8; 16] =
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

    pub const HI_PATH: Option<&str> = Some("hi");
    pub const UNREL_PATH: Option<&str> = Some("unrelated");
}
