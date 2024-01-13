// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::{any, Arbitrary};

use std::marker::PhantomData;
use std::mem;

// NoBackVec implements an abstraction of the Vector library which tracks only
// the length of the vector. It does not contain a backing store which implies
// that writes only increment the length and all reads return a non-deterministic
// value.
//
// This abstraction is particularly effective for use cases where the customer
// code only cares about the length of the vector. All length queries are
// fast because the solver does not have to reason about the memory model at all.
//
// This abstraction has several limitations however. Since it does not model any
// memory, defining general methods which operate on the values of the vector is
// hard and in some cases, unsound. Please see the README.md for a more in-depth
// discussion of potential improvements to this abstraction.
//
// NOTE: It would also be difficult to soundly model a Vector where the contained
// type has a non-trivial drop method defined for it.

const DEFAULT_CAPACITY: usize = 1073741824;

// The Vec structure here models the length and the capacity.
pub struct Vec<T> {
    len: usize,
    capacity: usize,
    // We use a _marker variable since we want the Vector to be generic over type
    // T. It is a zero-sized type which is used to mark things such that they act
    // like they own a T.
    _marker: PhantomData<T>,
}

impl<T: Arbitrary> Vec<T> {
    // The standard library Vec implementation calls reserve() to reserve
    // space for an additional element -> self.reserve(1). However, the
    // semantics of reserve() are ambiguous. reserve(num) allocates space for
    // "atleast num more elements of the containing type". The operation can
    // be found in function `grow_amortized()` in raw_vec.rs in the standard
    // library. The logic for choosing a new value is:
    // self.cap = max(self.cap * 2, self.len + additional)
    // We try to implement similar semantics here.
    fn grow(&mut self, additional: usize) {
        let new_len = self.len + additional;
        let grow_cap = self.capacity * 2;
        let new_capacity = if new_len > grow_cap { new_len } else { grow_cap };
        self.capacity = new_capacity;
    }
}

impl<T: Arbitrary> Vec<T> {
    pub fn new() -> Vec<T> {
        // By default, we create a vector with a high default capacity. An
        // important callout to make here is that it prevents us from discovering
        // buffer-overflow bugs since we will (most-likely) always have enough
        // space allocated additional to the required vec capacity.
        // NOTE: This is however not a concern for this abstaction.
        Vec { len: 0, capacity: DEFAULT_CAPACITY, _marker: Default::default() }
    }

    // Even though we dont model any memory, we can soundly model the capacity
    // of the allocation.
    pub fn with_capacity(capacity: usize) -> Self {
        Vec { len: 0, capacity: capacity, _marker: Default::default() }
    }

    pub fn push(&mut self, elem: T) {
        // Please refer to grow() for better understanding the semantics of reserve().
        if self.capacity == self.len {
            self.reserve(1);
        }

        assert!(self.capacity >= self.len);
        // We only increment the length of the vector disregarding the actual
        // element added to the Vector.
        self.len += 1;
    }

    // We check if there are any elements in the Vector. If not, we return a None
    // otherwise we return a nondeterministic value since we dont track any concrete
    // values in the Vector.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(any())
        }
    }

    pub fn append(&mut self, other: &mut Vec<T>) {
        let new_len = self.len + other.len;
        // Please refer to grow() for better understanding the semantics of grow().
        if self.capacity < new_len {
            self.reserve(other.len);
        }

        assert!(self.capacity >= new_len);
        // Drop all writes, increment the length of the Vector with the size
        // of the Vector which is appended.
        self.len = new_len;
    }

    // At whichever position we insert the new element into, the overall effect on
    // the abstraction is that the length increases by 1.
    pub fn insert(&mut self, index: usize, elem: T) {
        assert!(index <= self.len);

        self.len += 1;
    }

    // We only care that the index we are removing from lies somewhere as part of
    // the length of the Vector. The only effect on the abstraction is that the
    // length decreases by 1. In the case that it is a valid removal, we return a
    // nondeterministic value.
    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len);

        self.len -= 1;
        any()
    }

    pub fn extend<I: Iterator>(&mut self, iter: I)
    where
        I: Iterator<Item = T>,
    {
        // We first compute the length of the iterator.
        let mut iter_len = 0;
        for value in iter {
            iter_len += 1;
        }

        // Please refer to grow() for better understanding the semantics of grow().
        self.reserve(iter_len);
        self.len += iter_len;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    // Please refer to grow() for better understanding the semantics of reserve().
    pub fn reserve(&mut self, additional: usize) {
        self.grow(additional);
    }
}

// NoBackIter is a structure which implements Iterator suitable for NoBackVec. We
// only track the index values to the start and end of the iterator.
pub struct NoBackIter<T> {
    start: usize,
    end: usize,
    // Please refer to the NoBackvec definition to understand why PhantomData is used
    // here.
    _marker: PhantomData<T>,
}

impl<T> NoBackIter<T> {
    pub fn new(len: usize) -> Self {
        // By default, initialize the start to index 0 and end to the last index
        // of the Vector.
        NoBackIter { start: 0, end: len, _marker: Default::default() }
    }
}

impl<T: Arbitrary> Iterator for NoBackIter<T> {
    type Item = T;

    // Unless we are at the end of the array, return a nondeterministic value
    // wrapped around a Some.
    fn next(&mut self) -> Option<T> {
        if self.start == self.end { None } else { Some(any()) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.end - self.start;
        (len, Some(len))
    }
}

impl<T: Arbitrary> IntoIterator for Vec<T> {
    type Item = T;
    type IntoIter = NoBackIter<T>;

    fn into_iter(self) -> NoBackIter<T> {
        NoBackIter::new(self.len())
    }
}

// Here, we define the kani_vec! macro which behaves similar to the vec! macro
// found in the std prelude. If we try to override the vec! macro, we get error:
//
//     = note: `vec` could refer to a macro from prelude
//     note: `vec` could also refer to the macro defined here
//
// Relevant Zulip stream:
// https://rust-lang.zulipchat.com/#narrow/stream/122651-general/topic/Override.20prelude.20macro
//
// The workaround for now is to define a new macro. kani_vec! will initialize a new
// Vec based on its definition in this file. We support two types of initialization
// expressions:
//
// [ elem; count] -  initialize a Vector with element value `elem` occurring count times.
// [ elem1, elem2, ...] - initialize a Vector with elements elem1, elem2...
#[cfg(abs_type = "no-back")]
#[macro_export]
macro_rules! kani_vec {
  ( $val:expr ; $count:expr ) =>
    ({
      let mut result = Vec::new();
      let mut i: usize = 0;
      while i < $count {
        result.push($val);
        i += 1;
      }
      result
    });
  ( $( $xs:expr ),* ) => {
    {
      let mut result = Vec::new();
      $(
        result.push($xs);
      )*
      result
    }
  };
}
