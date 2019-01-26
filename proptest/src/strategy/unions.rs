//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::cmp::{max, min};
use core::u32;
use crate::std_facade::Vec;

#[cfg(not(feature="std"))]
use num_traits::float::FloatCore;

use crate::num::sample_uniform;
use crate::strategy::traits::*;
use crate::test_runner::*;

/// A **relative** `weight` of a particular `Strategy` corresponding to `T`
/// coupled with `T` itself. The weight is currently given in `u32`.
pub type W<T> = (u32, T);

/// A `Strategy` which picks from one of several delegate `Stragegy`s.
///
/// See `Strategy::prop_union()`.
#[derive(Clone, Debug)]
#[must_use = "strategies do nothing unless used"]
pub struct Union<T : Strategy> {
    options: Vec<W<T>>,
}

impl<T : Strategy> Union<T> {
    /// Create a strategy which selects uniformly from the given delegate
    /// strategies.
    ///
    /// When shrinking, after maximal simplification of the chosen element, the
    /// strategy will move to earlier options and continue simplification with
    /// those.
    ///
    /// ## Panics
    ///
    /// Panics if `options` is empty.
    pub fn new(options: impl IntoIterator<Item = T>) -> Self {
        let options: Vec<W<T>> = options.into_iter()
            .map(|v| (1, v)).collect();
        assert!(!options.is_empty());
        Self { options }
    }

    pub(crate) fn try_new<E>(it: impl Iterator<Item = Result<T, E>>)
                             -> Result<Self, E> {
        let options: Vec<W<T>> = it.map(|r| r.map(|v| (1, v)))
            .collect::<Result<_, _>>()?;

        assert!(!options.is_empty());
        Ok(Self { options })
    }

    /// Create a strategy which selects from the given delegate strategies.
    ///
    /// Each strategy is assigned a non-zero weight which determines how
    /// frequently that strategy is chosen. For example, a strategy with a
    /// weight of 2 will be chosen twice as frequently as one with a weight of
    /// 1\.
    ///
    /// ## Panics
    ///
    /// Panics if `options` is empty or any element has a weight of 0.
    ///
    /// Panics if the sum of the weights overflows a `u32`.
    pub fn new_weighted(options: Vec<W<T>>) -> Self {
        assert!(!options.is_empty());
        assert!(!options.iter().any(|&(w, _)| 0 == w),
                "Union option has a weight of 0");
        assert!(options.iter().map(|&(w, _)| u64::from(w)).sum::<u64>() <=
                u64::from(u32::MAX), "Union weights overflow u32");
        Self { options }
    }

    /// Add `other` as an additional alternate strategy with weight 1.
    pub fn or(mut self, other: T) -> Self {
        self.options.push((1, other));
        self
    }
}

fn pick_weighted<I : Iterator<Item = u32>>(runner: &mut TestRunner,
                                           weights1: I, weights2: I) -> usize {
    let sum = weights1.map(u64::from).sum();
    let weighted_pick = sample_uniform(runner, 0..sum);
    weights2.scan(0u64, |state, w| {
        *state += u64::from(w);
        Some(*state)
    }).filter(|&v| v <= weighted_pick).count()
}

impl<T : Strategy> Strategy for Union<T> {
    type Tree = UnionValueTree<T::Tree>;
    type Value = T::Value;

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        fn extract_weight<V>(&(w, _): &W<V>) -> u32 { w }

        let pick = pick_weighted(
            runner,
            self.options.iter().map(extract_weight::<T>),
            self.options.iter().map(extract_weight::<T>));

        let mut options = Vec::with_capacity(pick);
        for option in &self.options[0..pick+1] {
            options.push(option.1.new_tree(runner)?);
        }

        Ok(UnionValueTree { options, pick, min_pick: 0, prev_pick: None })
    }
}

/// `ValueTree` corresponding to `Union`.
#[derive(Clone, Debug)]
pub struct UnionValueTree<T : ValueTree> {
    options: Vec<T>,
    pick: usize,
    min_pick: usize,
    prev_pick: Option<usize>,
}

macro_rules! access_vec {
    ([$($muta:tt)*] $dst:ident = $this:expr, $ix:expr, $body:block) => {{
        let $dst = &$($muta)* $this.options[$ix];
        $body
    }}
}

macro_rules! union_value_tree_body {
    ($typ:ty, $access:ident) => {
        type Value = $typ;

        fn current(&self) -> Self::Value {
            $access!([] opt = self, self.pick, {
                opt.current()
            })
        }

        fn simplify(&mut self) -> bool {
            if $access!([mut] opt = self, self.pick, { opt.simplify() }) {
                self.prev_pick = None;
                true
            } else if self.pick > self.min_pick {
                self.prev_pick = Some(self.pick);
                self.pick -= 1;
                true
            } else {
                false
            }
        }

        fn complicate(&mut self) -> bool {
            if let Some(pick) = self.prev_pick {
                self.pick = pick;
                self.min_pick = pick;
                self.prev_pick = None;
                true
            } else {
                $access!([mut] opt = self, self.pick, { opt.complicate() })
            }
        }
    }
}

impl<T : ValueTree> ValueTree for UnionValueTree<T> {
    union_value_tree_body!(T::Value, access_vec);
}

macro_rules! def_access_tuple {
    ($b:tt $name:ident, $($n:tt)*) => {
        macro_rules! $name {
            ([$b($b muta:tt)*] $b dst:ident = $b this:expr,
             $b ix:expr, $b body:block) => {
                match $b ix {
                    0 => {
                        let $b dst = &$b($b muta)* $b this.options.0;
                        $b body
                    },
                    $(
                        $n => {
                            if let Some(ref $b($b muta)* $b dst) =
                                $b this.options.$n
                            {
                                $b body
                            } else {
                                panic!("TupleUnion tried to access \
                                        uninitialised slot {}", $n)
                            }
                        },
                    )*
                    _ => panic!("TupleUnion tried to access out-of-range \
                                 slot {}", $b ix),
                }
            }
        }
    }
}

def_access_tuple!($ access_tuple2, 1);
def_access_tuple!($ access_tuple3, 1 2);
def_access_tuple!($ access_tuple4, 1 2 3);
def_access_tuple!($ access_tuple5, 1 2 3 4);
def_access_tuple!($ access_tuple6, 1 2 3 4 5);
def_access_tuple!($ access_tuple7, 1 2 3 4 5 6);
def_access_tuple!($ access_tuple8, 1 2 3 4 5 6 7);
def_access_tuple!($ access_tuple9, 1 2 3 4 5 6 7 8);
def_access_tuple!($ access_tupleA, 1 2 3 4 5 6 7 8 9);

/// Similar to `Union`, but internally uses a tuple to hold the strategies.
///
/// This allows better performance than vanilla `Union` since one does not need
/// to resort to boxing and dynamic dispatch to handle heterogeneous
/// strategies.
#[must_use = "strategies do nothing unless used"]
#[derive(Clone, Copy, Debug)]
pub struct TupleUnion<T>(T);

impl<T> TupleUnion<T> {
    /// Wrap `tuple` in a `TupleUnion`.
    ///
    /// The struct definition allows any `T` for `tuple`, but to be useful, it
    /// must be a 2- to 10-tuple of `(u32, impl Strategy)` pairs where all
    /// strategies ultimately produce the same value. Each `u32` indicates the
    /// relative weight of its corresponding strategy.
    /// You may use `W<S>` as an alias for `(u32, S)`.
    ///
    /// Using this constructor directly is discouraged; prefer to use
    /// `prop_oneof!` since it is generally clearer.
    pub fn new(tuple: T) -> Self {
        TupleUnion(tuple)
    }
}

macro_rules! tuple_union {
    ($($gen:ident $ix:tt)*) => {
        impl<A : Strategy, $($gen: Strategy<Value = A::Value>),*>
        Strategy for TupleUnion<(W<A>, $(W<$gen>),*)> {
            type Tree = TupleUnionValueTree<
                (A::Tree, $(Option<$gen::Tree>),*)>;
            type Value = A::Value;

            fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
                let weights = [((self.0).0).0, $(((self.0).$ix).0),*];
                let pick = pick_weighted(runner, weights.iter().cloned(),
                                         weights.iter().cloned());

                Ok(TupleUnionValueTree {
                    options: (
                        ((self.0).0).1.new_tree(runner)?,
                        $(if $ix <= pick {
                            Some(((self.0).$ix).1.new_tree(runner)?)
                        } else {
                            None
                        }),*),
                    pick: pick,
                    min_pick: 0,
                    prev_pick: None,
                })
            }
        }
    }
}

tuple_union!(B 1);
tuple_union!(B 1 C 2);
tuple_union!(B 1 C 2 D 3);
tuple_union!(B 1 C 2 D 3 E 4);
tuple_union!(B 1 C 2 D 3 E 4 F 5);
tuple_union!(B 1 C 2 D 3 E 4 F 5 G 6);
tuple_union!(B 1 C 2 D 3 E 4 F 5 G 6 H 7);
tuple_union!(B 1 C 2 D 3 E 4 F 5 G 6 H 7 I 8);
tuple_union!(B 1 C 2 D 3 E 4 F 5 G 6 H 7 I 8 J 9);

/// `ValueTree` type produced by `TupleUnion`.
#[derive(Clone, Copy, Debug)]
pub struct TupleUnionValueTree<T> {
    options: T,
    pick: usize,
    min_pick: usize,
    prev_pick: Option<usize>,
}

macro_rules! value_tree_tuple {
    ($access:ident, $($gen:ident)*) => {
        impl<A : ValueTree, $($gen: ValueTree<Value = A::Value>),*> ValueTree
        for TupleUnionValueTree<(A, $(Option<$gen>),*)> {
            union_value_tree_body!(A::Value, $access);
        }
    }
}

value_tree_tuple!(access_tuple2, B);
value_tree_tuple!(access_tuple3, B C);
value_tree_tuple!(access_tuple4, B C D);
value_tree_tuple!(access_tuple5, B C D E);
value_tree_tuple!(access_tuple6, B C D E F);
value_tree_tuple!(access_tuple7, B C D E F G);
value_tree_tuple!(access_tuple8, B C D E F G H);
value_tree_tuple!(access_tuple9, B C D E F G H I);
value_tree_tuple!(access_tupleA, B C D E F G H I J);

const WEIGHT_BASE: u32 = 0x8000_0000;

/// Convert a floating-point weight in the range (0.0,1.0) to a pair of weights
/// that can be used with `Union` and similar.
///
/// The first return value is the weight corresponding to `f`; the second
/// return value is the weight corresponding to `1.0 - f`.
///
/// This call does not make any guarantees as to what range of weights it may
/// produce, except that adding the two return values will never overflow a
/// `u32`. As such, it is generally not meaningful to combine any other weights
/// with the two returned.
///
/// ## Panics
///
/// Panics if `f` is not a real number between 0.0 and 1.0, both exclusive.
pub fn float_to_weight(f: f64) -> (u32, u32) {
    assert!(f > 0.0 && f < 1.0, "Invalid probability: {}", f);

    // Clamp to 1..WEIGHT_BASE-1 so that we never produce a weight of 0.
    let pos = max(1, min(WEIGHT_BASE - 1,
                         (f * f64::from(WEIGHT_BASE)).round() as u32));
    let neg = WEIGHT_BASE - pos;

    (pos, neg)
}

#[cfg(test)]
mod test {
    use crate::strategy::just::Just;
    use super::*;

    // FIXME(2018-06-01): figure out a way to run this test on no_std.
    // The problem is that the default seed is fixed and does not produce
    // enough passed tests. We need some universal source of non-determinism
    // for the seed, which is unlikely.
    #[cfg(feature = "std")]
    #[test]
    fn test_union() {
        let input = (10u32..20u32).prop_union(30u32..40u32);
        // Expect that 25% of cases pass (left input happens to be < 15, and
        // left is chosen as initial value). Of the 75% that fail, 50% should
        // converge to 15 and 50% to 30 (the latter because the left is beneath
        // the passing threshold).
        let mut passed = 0;
        let mut converged_low = 0;
        let mut converged_high = 0;
        for _ in 0..256 {
            let mut runner = TestRunner::default();
            let case = input.new_tree(&mut runner).unwrap();
            let result = runner.run_one(case, |v| {
                prop_assert!(v < 15);
                Ok(())
            });

            match result {
                Ok(true) => passed += 1,
                Err(TestError::Fail(_, 15)) => converged_low += 1,
                Err(TestError::Fail(_, 30)) => converged_high += 1,
                e => panic!("Unexpected result: {:?}", e),
            }
        }

        assert!(passed >= 32 && passed <= 96,
                "Bad passed count: {}", passed);
        assert!(converged_low >= 32 && converged_low <= 160,
                "Bad converged_low count: {}", converged_low);
        assert!(converged_high >= 32 && converged_high <= 160,
                "Bad converged_high count: {}", converged_high);
    }

    #[test]
    fn test_union_weighted() {
        let input = Union::new_weighted(vec![
            (1, Just(0usize)),
            (2, Just(1usize)),
            (1, Just(2usize)),
        ]);

        let mut counts = [0, 0, 0];
        let mut runner = TestRunner::default();
        for _ in 0..65536 {
            counts[input.new_tree(&mut runner).unwrap().current()] += 1;
        }

        println!("{:?}", counts);
        assert!(counts[0] > 0);
        assert!(counts[2] > 0);
        assert!(counts[1] > counts[0] * 3/2);
        assert!(counts[1] > counts[2] * 3/2);
    }

    #[test]
    fn test_union_sanity() {
        check_strategy_sanity(Union::new_weighted(vec![
            (1, 0i32..100),
            (2, 200i32..300),
            (1, 400i32..500),
        ]), None);
    }

    // FIXME(2018-06-01): See note on `test_union`.
    #[cfg(feature = "std")]
    #[test]
    fn test_tuple_union() {
        let input = TupleUnion::new(
            ((1, 10u32..20u32),
             (1, 30u32..40u32)));
        // Expect that 25% of cases pass (left input happens to be < 15, and
        // left is chosen as initial value). Of the 75% that fail, 50% should
        // converge to 15 and 50% to 30 (the latter because the left is beneath
        // the passing threshold).
        let mut passed = 0;
        let mut converged_low = 0;
        let mut converged_high = 0;
        for _ in 0..256 {
            let mut runner = TestRunner::default();
            let case = input.new_tree(&mut runner).unwrap();
            let result = runner.run_one(case, |v| {
                prop_assert!(v < 15);
                Ok(())
            });

            match result {
                Ok(true) => passed += 1,
                Err(TestError::Fail(_, 15)) => converged_low += 1,
                Err(TestError::Fail(_, 30)) => converged_high += 1,
                e => panic!("Unexpected result: {:?}", e),
            }
        }

        assert!(passed >= 32 && passed <= 96,
                "Bad passed count: {}", passed);
        assert!(converged_low >= 32 && converged_low <= 160,
                "Bad converged_low count: {}", converged_low);
        assert!(converged_high >= 32 && converged_high <= 160,
                "Bad converged_high count: {}", converged_high);
    }

    #[test]
    fn test_tuple_union_weighting() {
        let input = TupleUnion::new((
            (1, Just(0usize)),
            (2, Just(1usize)),
            (1, Just(2usize)),
        ));

        let mut counts = [0, 0, 0];
        let mut runner = TestRunner::default();
        for _ in 0..65536 {
            counts[input.new_tree(&mut runner).unwrap().current()] += 1;
        }

        println!("{:?}", counts);
        assert!(counts[0] > 0);
        assert!(counts[2] > 0);
        assert!(counts[1] > counts[0] * 3/2);
        assert!(counts[1] > counts[2] * 3/2);
    }

    #[test]
    fn test_tuple_union_all_sizes() {
        let mut runner = TestRunner::default();
        let r = 1i32..10;

        macro_rules! test {
            ($($part:expr),*) => {{
                let input = TupleUnion::new((
                    $((1, $part.clone())),*,
                    (1, Just(0i32))
                ));

                let mut pass = false;
                for _ in 0..1024 {
                    if 0 == input.new_tree(&mut runner).unwrap().current() {
                        pass = true;
                        break;
                    }
                }

                assert!(pass);
            }}
        }

        test!(r); // 2
        test!(r, r); // 3
        test!(r, r, r); // 4
        test!(r, r, r, r); // 5
        test!(r, r, r, r, r); // 6
        test!(r, r, r, r, r, r); // 7
        test!(r, r, r, r, r, r, r); // 8
        test!(r, r, r, r, r, r, r, r); // 9
        test!(r, r, r, r, r, r, r, r, r); // 10
    }

    #[test]
    fn test_tuple_union_sanity() {
        check_strategy_sanity(
            TupleUnion::new(((1, 0i32..100i32), (1, 200i32..1000i32),
                             (1, 2000i32..3000i32))),
            None);
    }
}
