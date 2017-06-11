//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;

use test_runner::*;

/// A strategy for producing arbitrary values of a given type.
pub trait Strategy {
    type Value : ValueTree;

    /// Generate a new value tree from the given runner.
    ///
    /// This may fail if there are constraints on the generated value and the
    /// generator is unable to produce anything that satisfies them. Any
    /// failure is wrapped in `TestError::Abort`.
    fn new_value
        (&self, runner: &mut TestRunner)
         -> Result<Self::Value, String>;

    fn boxed(self) -> BoxedStrategy<<Self::Value as ValueTree>::Value>
    where Self : Sized + 'static {
        Box::new(BoxedStrategyWrapper(self))
    }
}

/// A generated value and its associated shrinker.
///
/// Conceptually, a `ValueTree` represents a spectrum between a "minimally
/// complex" value and a starting, randomly-chosen value. For values such as
/// numbers, this can be thought of as a simple binary search, and this is how
/// the `ValueTree` state machine is defined.
///
/// The `ValueTree` state machine notionally has three fields: low, current,
/// and high. Initially, low is the "minimally complex" value for the type, and
/// high and current are both the initially chosen value. It can be queried for
/// its current state. When shrinking, the controlling code tries simplifying
/// the value one step. If the test failure still happens with the simplified
/// value, further simplification occurs. Otherwise, the code steps back up
/// towards the prior complexity. The main invariant here is that the "high"
/// value always corresponds to a failing test case.
pub trait ValueTree {
    type Value : fmt::Debug;

    /// Returns the current value.
    fn current(&self) -> Self::Value;
    /// Attempts to simplify the current value. Notionally, this sets the
    /// "high" value to the current value, and the current value to a "halfway
    /// point" between high and low, rounding towards low.
    ///
    /// Returns whether any state changed as a result of this call.
    fn simplify(&mut self) -> bool;
    /// Attempts to partially undo the last simplification. Notionally, this
    /// sets the "low" value to one plus the current value, and the current
    /// value to a "halfway point" between high and the new low, rounding
    /// towards low.
    ///
    /// Returns whether any state changed as a result of this call.
    fn complicate(&mut self) -> bool;
}

impl<T : ValueTree + ?Sized> ValueTree for Box<T> {
    type Value = T::Value;
    fn current(&self) -> Self::Value { (**self).current() }
    fn simplify(&mut self) -> bool { (**self).simplify() }
    fn complicate(&mut self) -> bool { (**self).complicate() }
}

pub type BoxedStrategy<T> = Box<Strategy<Value = Box<ValueTree<Value = T>>>>;

struct BoxedStrategyWrapper<T>(T);
impl<T : Strategy> Strategy for BoxedStrategyWrapper<T>
where T::Value : 'static {
    type Value = Box<ValueTree<Value = <T::Value as ValueTree>::Value>>;

    fn new_value(&self, runner: &mut TestRunner)
        -> Result<Self::Value, String>
    {
        Ok(Box::new(self.0.new_value(runner)?))
    }
}
