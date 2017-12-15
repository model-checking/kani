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

use rand::{Rng, SeedableRng, XorShiftRng};

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

    fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
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

/// `Strategy` perturbation adaptor.
///
/// See `Strategy::prop_perturb()`.
pub struct Perturb<S, F> {
    pub(super) source: S,
    pub(super) fun: Arc<F>,
}

impl<S : fmt::Debug, F> fmt::Debug for Perturb<S, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Perturb")
            .field("source", &self.source)
            .field("fun", &"<function>")
            .finish()
    }
}

impl<S : Clone, F> Clone for Perturb<S, F> {
    fn clone(&self) -> Self {
        Perturb {
            source: self.source.clone(),
            fun: Arc::clone(&self.fun),
        }
    }
}

impl<S : Strategy, O : fmt::Debug,
     F : Fn (ValueFor<S>, XorShiftRng) -> O>
Strategy for Perturb<S, F> {
    type Value = PerturbValueTree<S::Value, F>;

    fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
        let rng = XorShiftRng::from_seed(runner.rng().gen());

        self.source.new_value(runner).map(
            |v| PerturbValueTree {
                source: v,
                fun: Arc::clone(&self.fun),
                rng,
            })
    }
}

/// `ValueTree` perturbation adaptor.
///
/// See `Strategy::prop_perturb()`.
pub struct PerturbValueTree<S, F> {
    source: S,
    fun: Arc<F>,
    rng: XorShiftRng,
}

impl<S : fmt::Debug, F> fmt::Debug for PerturbValueTree<S, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PerturbValueTree")
            .field("source", &self.source)
            .field("fun", &"<function>")
            .field("rng", &self.rng)
            .finish()
    }
}

impl<S : Clone, F> Clone for PerturbValueTree<S, F> {
    fn clone(&self) -> Self {
        PerturbValueTree {
            source: self.source.clone(),
            fun: Arc::clone(&self.fun),
            rng: self.rng.clone(),
        }
    }
}

impl<S : ValueTree, O : fmt::Debug, F : Fn (S::Value, XorShiftRng) -> O>
ValueTree for PerturbValueTree<S, F> {
    type Value = O;

    fn current(&self) -> O {
        (self.fun)(self.source.current(), self.rng.clone())
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
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_map() {
        TestRunner::default()
            .run(&(0..10).prop_map(|v| v * 2), |&v| {
                assert!(0 == v % 2);
                Ok(())
            }).unwrap();
    }

    #[test]
    fn perturb_uses_same_rng_every_time() {
        let mut runner = TestRunner::default();
        let input = Just(1).prop_perturb(|v, mut rng| v + rng.next_u32());

        for _ in 0..16 {
            let value = input.new_value(&mut runner).unwrap();
            assert_eq!(value.current(), value.current());
        }
    }

    #[test]
    fn perturb_uses_varying_random_seeds() {
        let mut runner = TestRunner::default();
        let input = Just(1).prop_perturb(|v, mut rng| v + rng.next_u32());

        let mut seen = HashSet::new();
        for _ in 0..64 {
            seen.insert(input.new_value(&mut runner).unwrap().current());
        }

        assert_eq!(64, seen.len());
    }
}
