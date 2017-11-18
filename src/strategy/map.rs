//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;
use std::sync::Arc;

use strategy::traits::*;
use test_runner::*;

/// `Strategy` and `ValueTree` map adaptor.
///
/// See `Strategy::prop_map()`.
pub struct Map<S, F> {
    pub(super) source: S,
    pub(super) fun: Arc<F>,
}

impl<S : fmt::Debug, F> fmt::Debug for Map<S, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Map")
            .field("source", &self.source)
            .field("fun", &"<function>")
            .finish()
    }
}

impl<S : Clone, F> Clone for Map<S, F> {
    fn clone(&self) -> Self {
        Map {
            source: self.source.clone(),
            fun: Arc::clone(&self.fun),
        }
    }
}

impl<S : Strategy, O : fmt::Debug,
     F : Fn (ValueFor<S>) -> O>
Strategy for Map<S, F> {
    type Value = Map<S::Value, F>;

    fn new_value(&self, runner: &mut TestRunner)
                 -> Result<Self::Value, String> {
        self.source.new_value(runner).map(
            |v| Map { source: v, fun: Arc::clone(&self.fun) })
    }
}

impl<S : ValueTree, O : fmt::Debug, F : Fn (S::Value) -> O>
ValueTree for Map<S, F> {
    type Value = O;

    fn current(&self) -> O {
        (self.fun)(self.source.current())
    }

    fn simplify(&mut self) -> bool {
        self.source.simplify()
    }

    fn complicate(&mut self) -> bool {
        self.source.complicate()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_map() {
        TestRunner::new(Config::default())
            .run(&(0..10).prop_map(|v| v * 2), |&v| {
                assert!(0 == v % 2);
                Ok(())
            }).unwrap();
    }
}
