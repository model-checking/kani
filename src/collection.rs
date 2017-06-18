//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Strategies for generating `std::collections` of values.

use std::cmp::Ord;
use std::collections::*;
use std::hash::Hash;
use std::ops::Range;

use bit_set::BitSet;
use rand;
use rand::distributions::IndependentSample;

use strategy::*;
use test_runner::*;

/// Strategy to create `Vec`s with a length in a certain range.
///
/// Created by the `vec()` function in the same module.
#[derive(Clone, Debug)]
pub struct VecStrategy<T : Strategy> {
    element: T,
    size: Range<usize>,
}

/// Create a strategy to generate `Vec`s containing elements drawn from
/// `element` and with a size range given by `size`.
pub fn vec<T : Strategy>(element: T, size: Range<usize>)
                         -> VecStrategy<T> {
    VecStrategy {
        element: element,
        size: size,
    }
}

/// Create a strategy to generate `VecDeque`s containing elements drawn from
/// `element` and with a size range given by `size`.
pub fn vec_deque<T : Strategy + 'static>
    (element: T, size: Range<usize>)
     -> BoxedStrategy<VecDeque<<T::Value as ValueTree>::Value>>
{
    vec(element, size).prop_map(|v| v.into()).boxed()
}

/// Create a strategy to generate `LinkedList`s containing elements drawn from
/// `element` and with a size range given by `size`.
pub fn linked_list<T : Strategy + 'static>
    (element: T, size: Range<usize>)
     -> BoxedStrategy<LinkedList<<T::Value as ValueTree>::Value>>
{
    vec(element, size).prop_map(|v| v.into_iter().collect()).boxed()
}

/// Create a strategy to generate `BinaryHeap`s containing elements drawn from
/// `element` and with a size range given by `size`.
pub fn binary_heap<T : Strategy + 'static>
    (element: T, size: Range<usize>)
    -> BoxedStrategy<BinaryHeap<<T::Value as ValueTree>::Value>>
where <T::Value as ValueTree>::Value : Ord {
    vec(element, size).prop_map(|v| v.into()).boxed()
}

/// Create a strategy to generate `HashSet`s containing elements drawn from
/// `element` and with a size range given by `size`.
///
/// This strategy will implicitly do local rejects to ensure that the `HashSet`
/// has at least the minimum number of elements, in case `element` should
/// produce duplicate values.
pub fn hash_set<T : Strategy + 'static>
    (element: T, size: Range<usize>)
    -> BoxedStrategy<HashSet<<T::Value as ValueTree>::Value>>
where <T::Value as ValueTree>::Value : Hash + Eq {
    vec(element, size.clone())
        .prop_map(|v| v.into_iter().collect::<HashSet<_>>())
        .prop_filter("HashSet minimum size".to_owned(),
                     move |s| s.len() >= size.start)
        .boxed()
}

/// Create a strategy to generate `BTreeSet`s containing elements drawn from
/// `element` and with a size range given by `size`.
///
/// This strategy will implicitly do local rejects to ensure that the
/// `BTreeSet` has at least the minimum number of elements, in case `element`
/// should produce duplicate values.
pub fn btree_set<T : Strategy + 'static>
    (element: T, size: Range<usize>)
    -> BoxedStrategy<BTreeSet<<T::Value as ValueTree>::Value>>
where <T::Value as ValueTree>::Value : Ord {
    vec(element, size.clone())
        .prop_map(|v| v.into_iter().collect::<BTreeSet<_>>())
        .prop_filter("BTreeSet minimum size".to_owned(),
                     move |s| s.len() >= size.start)
        .boxed()
}

/// Create a strategy to generate `HashMap`s containing keys and values drawn
/// from `key` and `value` respectively, and with a size within the given
/// range.
///
/// This strategy will implicitly do local rejects to ensure that the `HashMap`
/// has at least the minimum number of elements, in case `key` should produce
/// duplicate values.
pub fn hash_map<K : Strategy + 'static, V : Strategy + 'static>
    (key: K, value: V, size: Range<usize>)
    -> BoxedStrategy<HashMap<<K::Value as ValueTree>::Value,
                             <V::Value as ValueTree>::Value>>
where <K::Value as ValueTree>::Value : Hash + Eq {
    vec((key, value), size.clone())
        .prop_map(|v| v.into_iter().collect::<HashMap<_,_>>())
        .prop_filter("HashMap minimum size".to_owned(),
                     move |m| m.len() >= size.start)
        .boxed()
}

/// Create a strategy to generate `BTreeMap`s containing keys and values drawn
/// from `key` and `value` respectively, and with a size within the given
/// range.
///
/// This strategy will implicitly do local rejects to ensure that the
/// `BTreeMap` has at least the minimum number of elements, in case `key`
/// should produce duplicate values.
pub fn btree_map<K : Strategy + 'static, V : Strategy + 'static>
    (key: K, value: V, size: Range<usize>)
    -> BoxedStrategy<BTreeMap<<K::Value as ValueTree>::Value,
                              <V::Value as ValueTree>::Value>>
where <K::Value as ValueTree>::Value : Ord {
    vec((key, value), size.clone())
        .prop_map(|v| v.into_iter().collect::<BTreeMap<_,_>>())
        .prop_filter("BTreeMap minimum size".to_owned(),
                     move |m| m.len() >= size.start)
        .boxed()
}

#[derive(Clone, Copy, Debug)]
enum Shrink {
    DeleteElement(usize),
    ShrinkElement(usize),
}

/// `ValueTree` corresponding to `VecStrategy`.
#[derive(Clone, Debug)]
pub struct VecValueTree<T : ValueTree> {
    elements: Vec<T>,
    included_elements: BitSet,
    min_size: usize,
    shrink: Shrink,
    prev_shrink: Option<Shrink>,
}

impl<T : Strategy> Strategy for VecStrategy<T> {
    type Value = VecValueTree<T::Value>;

    fn new_value(&self, runner: &mut TestRunner)
                 -> Result<Self::Value, String> {
        let max_size = rand::distributions::Range::new(
            self.size.start, self.size.end).ind_sample(runner.rng());
        let mut elements = Vec::with_capacity(max_size);
        while elements.len() < max_size {
            elements.push(self.element.new_value(runner)?);
        }

        Ok(VecValueTree {
            elements: elements,
            included_elements: (0..max_size).collect(),
            min_size: self.size.start,
            shrink: Shrink::DeleteElement(0),
            prev_shrink: None,
        })
    }
}

impl<T : ValueTree> ValueTree for VecValueTree<T> {
    type Value = Vec<T::Value>;

    fn current(&self) -> Vec<T::Value> {
        self.elements.iter().enumerate()
            .filter(|&(ix, _)| self.included_elements.contains(ix))
            .map(|(_, element)| element.current())
            .collect()
    }

    fn simplify(&mut self) -> bool {
        // The overall strategy here is to iteratively delete elements from the
        // list until we can do so no further, then to shrink each remaining
        // element in sequence.
        //
        // For `complicate()`, we simply undo the last shrink operation, if
        // there was any.
        if let Shrink::DeleteElement(ix) = self.shrink {
            // Can't delete an element if beyond the end of the vec or if it
            // would put us under the minimum length.
            if ix >= self.elements.len() ||
                self.included_elements.len() == self.min_size
            {
                self.shrink = Shrink::ShrinkElement(0);
            } else {
                self.included_elements.remove(ix);
                self.prev_shrink = Some(self.shrink);
                self.shrink = Shrink::DeleteElement(ix + 1);
                return true;
            }
        }

        while let Shrink::ShrinkElement(ix) = self.shrink {
            if ix >= self.elements.len() {
                // Nothing more we can do
                return false;
            }

            if !self.included_elements.contains(ix) {
                // No use shrinking something we're not including.
                self.shrink = Shrink::ShrinkElement(ix + 1);
            }

            if !self.elements[ix].simplify() {
                // Move on to the next element
                self.shrink = Shrink::ShrinkElement(ix + 1);
            } else {
                self.prev_shrink = Some(self.shrink);
                return true;
            }
        }

        panic!("Unexpected shrink state");
    }

    fn complicate(&mut self) -> bool {
        match self.prev_shrink {
            None => false,
            Some(Shrink::DeleteElement(ix)) => {
                // Undo the last item we deleted. Can't complicate any further,
                // so unset prev_shrink.
                self.included_elements.insert(ix);
                self.prev_shrink = None;
                true
            },
            Some(Shrink::ShrinkElement(ix)) => {
                if self.elements[ix].complicate() {
                    // Don't unset prev_shrink; we may be able to complicate
                    // again.
                    true
                } else {
                    // Can't complicate the last element any further.
                    self.prev_shrink = None;
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_vec() {
        let input = vec(1usize..20usize, 5..20);
        let mut num_successes = 0;

        for _ in 0..256 {
            let mut runner = TestRunner::new(Config::default());
            let case = input.new_value(&mut runner).unwrap();
            let start = case.current();
            // Has correct length
            assert!(start.len() >= 5 && start.len() < 20);
            // Has at least 2 distinct values
            assert!(start.iter().map(|&v| v).collect::<BitSet>().len() >= 2);

            let result = runner.run_one(case, |v| {
                if v.iter().map(|&v| v).sum::<usize>() < 9 {
                    Ok(())
                } else {
                    Err(TestCaseError::Fail("greater than 8".to_owned()))
                }
            });

            match result {
                Ok(true) => num_successes += 1,
                Err(TestError::Fail(_, value)) => {
                    // The minimal case always has between 5 (due to min
                    // length) and 9 (min element value = 1) elements, and
                    // always sums to exactly 9.
                    assert!(value.len() >= 5 && value.len() <= 9 &&
                            value.iter().map(|&v| v).sum::<usize>() == 9,
                            "Unexpected minimal value: {:?}", value);
                },
                e => panic!("Unexpected result: {:?}", e),
            }
        }

        assert!(num_successes < 256);
    }

    #[test]
    fn test_map() {
        // Only 8 possible keys
        let input = hash_map("[ab]{3}", "a", 2..3);
        let mut runner = TestRunner::new(Config::default());

        for _ in 0..256 {
            let v = input.new_value(&mut runner).unwrap().current();
            assert_eq!(2, v.len());
        }
    }

    #[test]
    fn test_set() {
        // Only 8 possible values
        let input = hash_set("[ab]{3}", 2..3);
        let mut runner = TestRunner::new(Config::default());

        for _ in 0..256 {
            let v = input.new_value(&mut runner).unwrap().current();
            assert_eq!(2, v.len());
        }
    }
}
