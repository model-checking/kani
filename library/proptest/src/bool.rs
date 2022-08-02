//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Strategies for generating `bool` values.

use crate::strategy::*;
use crate::test_runner::*;

/// The type of the `ANY` constant.
#[derive(Clone, Copy, Debug)]
pub struct Any(());

/// Generates boolean values by picking `true` or `false` uniformly.
///
/// Shrinks `true` to `false`.
pub const ANY: Any = Any(());

impl Strategy for Any {
    type Tree = BoolValueTree;
    type Value = bool;

    fn new_tree(&self, _: &mut TestRunner) -> NewTree<Self> {
        Ok(BoolValueTree::new(kani::any()))
    }
}

/// Generates boolean values by picking `true` with the given `probability`
/// (1.0 = always true, 0.0 = always false).
///
/// Shrinks `true` to `false`.
pub fn weighted(probability: f64) -> Weighted {
    Weighted(probability)
}

/// The return type from `weighted()`.
#[must_use = "strategies do nothing unless used"]
#[derive(Clone, Copy, Debug)]
pub struct Weighted(f64);

impl Strategy for Weighted {
    type Tree = BoolValueTree;
    type Value = bool;

    fn new_tree(&self, _: &mut TestRunner) -> NewTree<Self> {
        if self.0 >= 1.0 {
            Ok(BoolValueTree::new(true))
        } else if self.0 <= 0.0 {
            Ok(BoolValueTree::new(false))
        } else {
            Ok(BoolValueTree::new(kani::any()))
        }
    }
}

/// The `ValueTree` to shrink booleans to false.
#[derive(Clone, Copy, Debug)]
pub struct BoolValueTree {
    current: bool,
}

impl BoolValueTree {
    fn new(current: bool) -> Self {
        BoolValueTree { current,  }
    }
}

impl ValueTree for BoolValueTree {
    type Value = bool;

    fn current(&self) -> bool {
        self.current
    }
    fn simplify(&mut self) -> bool {
        false
    }
    fn complicate(&mut self) -> bool {
        false
    }
}

