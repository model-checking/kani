//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Strategies for generating values by taking samples of collections.
//!
//! Note that the strategies in this module are not native combinators; that
//! is, the input collection is not itself a strategy, but is rather fixed when
//! the strategy is created.

use std::borrow::Cow;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use bit_set::BitSet;

use bits::{self, BitSetValueTree, SampledBitSetStrategy};
use num;
use strategy::*;
use test_runner::*;

/// Sample subsequences whose size are within `size` from the given collection
/// `values`.
///
/// A subsequence is a subset of the elements in a collection in the order they
/// occur in that collection. The elements are not chosen to be contiguous.
///
/// This is roughly analogous to `rand::sample`, except that it guarantees that
/// the order is preserved.
///
/// `values` may be a static slice or a `Vec`.
///
/// ## Panics
///
/// Panics if the maximum size implied by `size` is larger than the size of
/// `values`.
///
/// Panics if `size` is a zero-length range.
pub fn subsequence<T, A>(values: A, size: Range<usize>) -> Subsequence<T>
where A : 'static + Into<Cow<'static, [T]>>, T : Clone + 'static {
    let values = values.into();
    let len = values.len();

    assert!(size.start != size.end, "Zero-length range passed to subsequence");
    assert!(size.end <= len + 1,
            "Maximum size of subsequence {} exceeds length of input {}",
            size.end, len);
    Subsequence {
        values: Arc::new(values),
        bit_strategy: bits::bitset::sampled(size, 0..len),
    }
}

/// Strategy to generate `Vec`s by sampling a subsequence from another
/// collection.
///
/// This is created by the `subsequence` function in the same module.
#[derive(Debug, Clone)]
pub struct Subsequence<T : Clone + 'static> {
    values: Arc<Cow<'static, [T]>>,
    bit_strategy: SampledBitSetStrategy<BitSet>,
}

impl<T : fmt::Debug + Clone + 'static> Strategy for Subsequence<T> {
    type Value = SubsequenceValueTree<T>;

    fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
        Ok(SubsequenceValueTree {
            values: Arc::clone(&self.values),
            inner: self.bit_strategy.new_value(runner)?,
        })
    }
}

/// `ValueTree` type for `Subsequence`.
#[derive(Debug, Clone)]
pub struct SubsequenceValueTree<T : Clone + 'static> {
    values: Arc<Cow<'static, [T]>>,
    inner: BitSetValueTree<BitSet>,
}

impl<T : fmt::Debug + Clone + 'static> ValueTree for SubsequenceValueTree<T> {
    type Value = Vec<T>;

    fn current(&self) -> Self::Value {
        self.inner.current().into_iter().map(
            |ix| self.values[ix].clone()).collect()
    }

    fn simplify(&mut self) -> bool {
        self.inner.simplify()
    }

    fn complicate(&mut self) -> bool {
        self.inner.complicate()
    }
}


#[derive(Debug, Clone)]
struct SelectMapFn<T : Clone + 'static>(Arc<Cow<'static, [T]>>);

impl<T : fmt::Debug + Clone + 'static> statics::MapFn<usize>
for SelectMapFn<T> {
    type Output = T;

    fn apply(&self, ix: usize) -> T {
        self.0[ix].clone()
    }
}

opaque_strategy_wrapper! {
    /// Strategy to produce one value from a fixed collection of options.
    ///
    /// Created by the `select()` in the same module.
    #[derive(Clone, Debug)]
    pub struct Select[<T>][where T : Clone + fmt::Debug + 'static](
        statics::Map<Range<usize>, SelectMapFn<T>>)
        -> SelectValueTree<T>;
    /// `ValueTree` corresponding to `Select`.
    #[derive(Clone, Debug)]
    pub struct SelectValueTree[<T>][where T : Clone + fmt::Debug + 'static](
        statics::Map<num::usize::BinarySearch, SelectMapFn<T>>)
        -> T;
}

/// Create a strategy which uniformly selects one value from `values`.
///
/// `values` should be a `&'static [T]` or a `Vec<T>`, or potentially another
/// type that can be coerced to `Cow<'static,[T]>`.
///
/// This is largely equivalent to making a `Union` of a bunch of `Just`
/// strategies, but is substantially more efficient and shrinks by binary
/// search.
pub fn select<T, A>(values: A) -> Select<T>
where A : 'static + Into<Cow<'static, [T]>>, T : Clone + fmt::Debug + 'static {
    let cow = values.into();

    Select(statics::Map::new(
        0..cow.len(), SelectMapFn(Arc::new(cow))))
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn sample_slice() {
        static VALUES: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 7];
        let mut size_counts = [0; 8];
        let mut value_counts = [0; 8];

        let mut runner = TestRunner::default();
        let input = subsequence(VALUES, 3..7);

        for _ in 0..2048 {
            let value = input.new_value(&mut runner).unwrap().current();
            // Generated the correct number of items
            assert!(value.len() >= 3 && value.len() < 7);
            // Chose distinct items
            assert_eq!(value.len(),
                       value.iter().cloned().collect::<HashSet<_>>().len());
            // Values are in correct order
            let mut sorted = value.clone();
            sorted.sort();
            assert_eq!(sorted, value);

            size_counts[value.len()] += 1;

            for value in value {
                value_counts[value] += 1;
            }
        }

        for i in 3..7 {
            assert!(size_counts[i] >= 256 && size_counts[i] < 1024,
                    "size {} was chosen {} times", i, size_counts[i]);
        }

        for (ix, &v) in value_counts.iter().enumerate() {
            assert!(v >= 1024 && v < 1500,
                    "Value {} was chosen {} times", ix, v);
        }
    }

    #[test]
    fn sample_vec() {
        // Just test that the types work out
        let values = vec![0, 1, 2, 3, 4];

        let mut runner = TestRunner::default();
        let input = subsequence(values, 1..3);

        let _ = input.new_value(&mut runner).unwrap().current();
    }

    #[test]
    fn test_select() {
        let values = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let mut counts = [0; 8];

        let mut runner = TestRunner::default();
        let input = select(values);

        for _ in 0..1024 {
            counts[input.new_value(&mut runner).unwrap().current()] += 1;
        }

        for (ix, &count) in counts.iter().enumerate() {
            assert!(count >= 64 && count < 256,
                    "Generated value {} {} times", ix, count);
        }
    }

    #[test]
    fn test_sample_sanity() {
        check_strategy_sanity(subsequence(vec![0, 1, 2, 3, 4], 1..3), None);
    }

    #[test]
    fn test_select_sanity() {
        check_strategy_sanity(select(vec![0, 1, 2, 3, 4]), None);
    }
}
