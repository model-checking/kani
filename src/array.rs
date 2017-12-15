//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Support for strategies producing fixed-length arrays.
//!
//! There is no explicit type for array strategies; instead, simply make an
//! array containing the desired strategy(ies) of the desired length.
//!
//! General implementations are available for sizes 1 through 32.

use strategy::*;
use test_runner::*;

/// A `ValueTree` operating over a fixed-size array.
#[derive(Clone, Copy, Debug)]
pub struct ArrayValueTree<T> {
    tree: T,
    shrinker: usize,
    last_shrinker: Option<usize>,
}

macro_rules! small_array {
    ($n:tt : $($ix:expr),*) => {
        impl<S : Strategy> Strategy for [S;$n] {
            type Value = ArrayValueTree<[S::Value;$n]>;

            fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
                Ok(ArrayValueTree {
                    tree: [$(self[$ix].new_value(runner)?,)*],
                    shrinker: 0,
                    last_shrinker: None,
                })
            }
        }

        impl<T : ValueTree> ValueTree for ArrayValueTree<[T;$n]> {
            type Value = [T::Value;$n];

            fn current(&self) -> [T::Value;$n] {
                [$(self.tree[$ix].current(),)*]
            }

            fn simplify(&mut self) -> bool {
                while self.shrinker < $n {
                    if self.tree[self.shrinker].simplify() {
                        self.last_shrinker = Some(self.shrinker);
                        return true;
                    } else {
                        self.shrinker += 1;
                    }
                }

                false
            }

            fn complicate(&mut self) -> bool {
                if let Some(shrinker) = self.last_shrinker {
                    self.shrinker = shrinker;
                    if self.tree[shrinker].complicate() {
                        true
                    } else {
                        self.last_shrinker = None;
                        false
                    }
                } else {
                    false
                }
            }
        }
    }
}

small_array!(1: 0);
small_array!(2: 0, 1);
small_array!(3: 0, 1, 2);
small_array!(4: 0, 1, 2, 3);
small_array!(5: 0, 1, 2, 3, 4);
small_array!(6: 0, 1, 2, 3, 4, 5);
small_array!(7: 0, 1, 2, 3, 4, 5, 6);
small_array!(8: 0, 1, 2, 3, 4, 5, 6, 7);
small_array!(9: 0, 1, 2, 3, 4, 5, 6, 7, 8);
small_array!(10: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
small_array!(11: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
small_array!(12: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11);
small_array!(13: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12);
small_array!(14: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13);
small_array!(15: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14);
small_array!(16: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
small_array!(17: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
small_array!(18: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17);
small_array!(19: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18);
small_array!(20: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18, 19);
small_array!(21: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18, 19, 20);
small_array!(22: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18, 19, 20, 21);
small_array!(23: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18, 19, 20, 21, 22);
small_array!(24: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18, 19, 20, 21, 22, 23);
small_array!(25: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18, 19, 20, 21, 22, 23, 24);
small_array!(26: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18, 19, 20, 21, 22, 23, 24, 25);
small_array!(27: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18, 19, 20, 21, 22, 23, 24, 25, 26);
small_array!(28: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18, 19, 20, 21, 22, 23, 24, 25, 26, 27);
small_array!(29: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28);
small_array!(30: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29);
small_array!(31: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30);
small_array!(32: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn shrinks_fully_ltr() {
        fn pass(a: [i32;2]) -> bool {
            a[0] * a[1] <= 9
        }

        let input = [0..32, 0..32];
        let mut runner = TestRunner::default();

        let mut cases_tested = 0;
        for _ in 0..256 {
            // Find a failing test case
            let mut case = input.new_value(&mut runner).unwrap();
            if pass(case.current()) { continue; }

            loop {
                if pass(case.current()) {
                    if !case.complicate() { break; }
                } else {
                    if !case.simplify() { break; }
                }
            }

            let last = case.current();
            assert!(!pass(last));
            // Maximally shrunken
            assert!(pass([last[0] - 1, last[1]]));
            assert!(pass([last[0], last[1] - 1]));

            cases_tested += 1;
        }

        assert!(cases_tested > 32, "Didn't find enough test cases");
    }

    #[test]
    fn test_sanity() {
        check_strategy_sanity([(0i32..1000),(1i32..1000)], None);
    }
}
