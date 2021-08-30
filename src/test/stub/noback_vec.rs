// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern crate libc;

use std::marker::PhantomData;
use std::mem;
use __nondet;

// Abstraction which tracks only the length of the vector and does not contain
// a backing store.
//
// All reads return a non-deterministic value and writes only increment the 
// length of the vector.
pub struct Vec<T> {
    len: usize,
    capacity: usize,
    _marker: PhantomData<T>,
}

impl<T> Vec<T> {
    pub fn new() -> Vec<T> {
        Vec {
            len: 0,
            capacity: 0,
            _marker: Default::default()
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Vec {
            len: 0,
            capacity: capacity,
            _marker: Default::default()
        }
    }

    pub fn push(&mut self, elem: T) {
        self.len += 1;

        if self.capacity < self.len {
            self.capacity = self.len;
        }

        assert!(self.capacity >= self.len);
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            Some(__nondet::<T>())
        }
    }

    pub fn append(&mut self, other: &mut Vec<T>) {
        let new_len = self.len + other.len;
        if self.capacity < new_len {
            self.capacity = new_len;
        }

        assert!(self.capacity >= new_len);
        self.len = new_len;
    }

    pub fn insert(&mut self, index: usize, elem: T) {
        assert!(index <= self.len);

        self.len += 1;
    }

    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len);

        __nondet::<T>()
    }

    pub fn extend<I: Iterator>(&mut self, iter: I) where I: Iterator<Item = T> {
        for value in iter {
            self.len += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity9(&self) -> usize {
        self.capacity
    }

    pub fn reserve(&mut self, _: usize) {
    }

    pub fn set_len(&mut self, new_len: usize) {
        self.len = new_len;
    }
}

pub struct NoBackIter<T> {
    start: usize,
    end: usize,
    _marker: PhantomData<T>
}

impl<T> NoBackIter<T> {
    pub fn new(len: usize) -> Self {
        NoBackIter {
            start: 0,
            end: len,
            _marker: Default::default()
        }
    }
}

impl<T> Iterator for NoBackIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.start == self.end {
            None
        } else {
            Some(__nondet::<T>())
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let elem_size = mem::size_of::<T>();
        let len = (self.end - self.start) / if elem_size == 0 { 1 } else { elem_size };
        (len, Some(len))
    }
}

impl<T> IntoIterator for Vec<T> {
    type Item = T;
    type IntoIter = NoBackIter<T>;

    fn into_iter(self) -> NoBackIter<T> {
        NoBackIter::new(self.len())
    }
}

#[cfg(abs_type = "no-back")]
#[macro_export]
macro_rules! rmc_vec {
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
