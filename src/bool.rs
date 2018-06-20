//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Strategies for generating `bool` values.

use strategy::*;
use test_runner::*;

use rand::Rng;

/// The type of the `ANY` constant.
#[derive(Clone, Copy, Debug)]
pub struct Any(());

/// Generates boolean values by picking `true` or `false` uniformly.
///
/// Shrinks `true` to `false`.
pub const ANY: Any = Any(());

impl Strategy for Any {
    type Value = BoolValueTree;

    fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
        Ok(BoolValueTree::new(runner.rng().gen()))
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
#[derive(Clone, Copy, Debug)]
pub struct Weighted(f64);

impl Strategy for Weighted {
    type Value = BoolValueTree;

    fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
        Ok(BoolValueTree::new(runner.rng().next_f64() < self.0))
    }
}

/// The `ValueTree` to shrink booleans to false.
#[derive(Clone, Copy, Debug)]
pub struct BoolValueTree {
    current: bool,
    state: ShrinkState,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ShrinkState {
    Untouched, Simplified, Final
}

impl BoolValueTree {
    fn new(current: bool) -> Self {
        BoolValueTree { current, state: ShrinkState::Untouched }
    }
}

impl ValueTree for BoolValueTree {
    type Value = bool;

    fn current(&self) -> bool { self.current }
    fn simplify(&mut self) -> bool {
        match self.state {
            ShrinkState::Untouched if self.current => {
                self.current = false;
                self.state = ShrinkState::Simplified;
                true
            },

            ShrinkState::Untouched | ShrinkState::Simplified |
            ShrinkState::Final => {
                self.state = ShrinkState::Final;
                false
            },
        }
    }
    fn complicate(&mut self) -> bool {
        match self.state {
            ShrinkState::Untouched | ShrinkState::Final => {
                self.state = ShrinkState::Final;
                false
            },

            ShrinkState::Simplified => {
                self.current = true;
                self.state = ShrinkState::Final;
                true
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sanity() {
        check_strategy_sanity(ANY, None);
    }

    #[test]
    fn shrinks_properly() {
        let mut runner = TestRunner::default();

        for _ in 0..256 {
            let mut tree = ANY.new_value(&mut runner).unwrap();

            if tree.current() {
                assert!(tree.simplify());
                assert!(!tree.current());
                assert!(!tree.clone().simplify());
                assert!(tree.complicate());
                assert!(!tree.clone().complicate());
                assert!(tree.current());
                assert!(!tree.simplify());
                assert!(tree.current());
            } else {
                assert!(!tree.clone().simplify());
                assert!(!tree.clone().complicate());
            }
        }
    }
}
