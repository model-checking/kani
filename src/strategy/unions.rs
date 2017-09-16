//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::u32;

use rand;
use rand::distributions::IndependentSample;

use strategy::traits::*;
use test_runner::*;

/// A `Strategy` which picks from one of several delegate `Stragegy`s.
///
/// See `Strategy::prop_union()`.
#[derive(Clone, Debug)]
pub struct Union<T : Strategy> {
    options: Vec<(u32,T)>,
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
    pub fn new<I : IntoIterator<Item = T>>(options: I) -> Self {
        let options: Vec<(u32,T)> = options.into_iter()
            .map(|v| (1, v)).collect();
        assert!(options.len() > 0);

        Union { options }
    }

    /// Create a strategy which selects from the given delegate strategies.
    ///
    /// Each strategy is assigned a non-zero weight which determines how
    /// frequently that strategy is chosen. For example, a strategy with a
    /// weight of 2 will be chosen twice as frequently as one with a weight of
    /// 1.
    ///
    /// ## Panics
    ///
    /// Panics if `options` is empty or any element has a weight of 0.
    ///
    /// Panics if the sum of the weights overflows a `u32`.
    pub fn new_weighted(options: Vec<(u32,T)>) -> Self {
        assert!(options.len() > 0);
        assert!(!options.iter().any(|&(w, _)| 0 == w),
                "Union option has a weight of 0");
        assert!(options.iter().map(|&(w, _)| w as u64).sum::<u64>() <=
                u32::MAX as u64, "Union weights overflow u32");
        Union { options }
    }

    /// Add `other` as an additional alternate strategy with weight 1.
    pub fn or(mut self, other: T) -> Self {
        self.options.push((1, other));
        self
    }
}

impl<T : Strategy> Strategy for Union<T> {
    type Value = UnionValueTree<T::Value>;

    fn new_value(&self, runner: &mut TestRunner)
                 -> Result<Self::Value, String> {
        let weight_sum: u32 = self.options.iter().map(|&(w,_)| w).sum();
        let weighted_pick = rand::distributions::Range::new(0, weight_sum)
            .ind_sample(runner.rng());
        let pick = self.options.iter().map(|&(w,_)| w)
            .scan(0, |state, w| {
                *state += w;
                Some(*state)
            }).filter(|&v| v <= weighted_pick).count();

        let mut options = Vec::with_capacity(pick);
        for option in &self.options[0..pick+1] {
            options.push(option.1.new_value(runner)?);
        }

        Ok(UnionValueTree {
            options: options,
            pick: pick,
            min_pick: 0,
            prev_pick: None,
        })
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

impl<T : ValueTree> ValueTree for UnionValueTree<T> {
    type Value = T::Value;

    fn current(&self) -> T::Value {
        self.options[self.pick].current()
    }

    fn simplify(&mut self) -> bool {
        if self.options[self.pick].simplify() {
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
            self.options[self.pick].complicate()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
            let mut runner = TestRunner::new(Config::default());
            let case = input.new_value(&mut runner).unwrap();
            let result = runner.run_one(case, |&v| if v < 15 {
                Ok(())
            } else {
                Err(TestCaseError::Fail("fail".to_owned()))
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
        let mut runner = TestRunner::new(Config::default());
        for _ in 0..65536 {
            counts[input.new_value(&mut runner).unwrap().current()] += 1;
        }

        println!("{:?}", counts);
        assert!(counts[0] > 0);
        assert!(counts[2] > 0);
        assert!(counts[1] > counts[0] * 3/2);
        assert!(counts[1] > counts[2] * 3/2);
    }
}
